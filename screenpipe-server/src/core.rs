use crate::db_types::Speaker;
use crate::{DatabaseManager, VideoCapture};
use anyhow::Result;
use futures::future::join_all;
use screenpipe_audio::{
    record_and_transcribe, AudioInput, AudioTranscriptionEngine, TranscriptionResult,
};
use screenpipe_audio::{start_realtime_recording, AudioStream};
use screenpipe_core::pii_removal::remove_pii;
use screenpipe_core::{DeviceType, Language};
use screenpipe_vision::core::{RealtimeVisionEvent, WindowOcr};
use screenpipe_vision::OcrEngine;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

pub struct RecordingConfig {
    pub output_path: Arc<String>,
    pub fps: f64,
    pub audio_chunk_duration: Duration,
    pub video_chunk_duration: Duration,
    pub use_pii_removal: bool,
    pub languages: Arc<[Language]>,
    pub capture_unfocused_windows: bool,
}

pub struct AudioConfig {
    pub disabled: bool,
    pub transcription_engine: Arc<AudioTranscriptionEngine>,
    pub realtime_enabled: bool,
    pub deepgram_api_key: Option<String>,
    pub whisper_sender: crossbeam::channel::Sender<AudioInput>,
    pub whisper_receiver: crossbeam::channel::Receiver<TranscriptionResult>,
}

pub struct VisionConfig {
    pub disabled: bool,
    pub ocr_engine: Arc<OcrEngine>,
    pub ignored_windows: Arc<[String]>,
    pub include_windows: Arc<[String]>,
    pub control: Arc<AtomicBool>,
    pub realtime_sender: Arc<tokio::sync::broadcast::Sender<RealtimeVisionEvent>>,
}

#[allow(clippy::too_many_arguments)]
pub async fn start_continuous_recording(
    db: Arc<DatabaseManager>,
    recording_config: RecordingConfig,
    audio_config: AudioConfig,
    vision_config: VisionConfig,
    vision_handle: &Handle,
    audio_handle: &Handle,
    devices: Arc<[DeviceType]>,
) -> Result<()> {
    let video_tasks = if !vision_config.disabled {
        devices
            .iter()
            .filter(|device| matches!(device, DeviceType::Vision(_)))
            .map(|device| {
                let db_manager_video = Arc::clone(&db);
                let output_path_video = Arc::clone(&recording_config.output_path);
                let is_running_video = Arc::clone(&vision_config.control);
                let ocr_engine = Arc::clone(&vision_config.ocr_engine);
                let realtime_vision_sender_clone = vision_config.realtime_sender.clone();
                let ignored_windows_clone = vision_config.ignored_windows.clone();
                let include_windows_clone = vision_config.include_windows.clone();

                let languages = recording_config.languages.clone();

                let monitor_id = match device {
                    DeviceType::Vision(id) => *id,
                    _ => panic!("Expected a vision device"),
                };

                debug!("Starting video recording for monitor {}", monitor_id);
                vision_handle.spawn(async move {
                    record_video(
                        db_manager_video,
                        output_path_video,
                        recording_config.fps,
                        is_running_video,
                        ocr_engine,
                        monitor_id,
                        recording_config.use_pii_removal,
                        ignored_windows_clone,
                        include_windows_clone,
                        recording_config.video_chunk_duration,
                        languages.clone(),
                        recording_config.capture_unfocused_windows,
                        realtime_vision_sender_clone,
                    )
                    .await
                })
            })
            .collect::<Vec<_>>()
    } else {
        vec![vision_handle.spawn(async move {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(())
        })]
    };

    let whisper_sender_clone = audio_config.whisper_sender.clone();
    let db_manager_audio = Arc::clone(&db);

    let audio_task = if !audio_config.disabled {
        audio_handle.spawn(async move {
            record_audio(
                db_manager_audio,
                recording_config.audio_chunk_duration,
                audio_config.whisper_sender,
                audio_config.whisper_receiver,
                audio_config.transcription_engine,
                audio_config.realtime_enabled,
                devices,
                recording_config.languages,
                audio_config.deepgram_api_key,
            )
            .await
        })
    } else {
        audio_handle.spawn(async move {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(())
        })
    };

    // Join all video tasks
    let video_results = join_all(video_tasks);

    // Handle any errors from the tasks
    for (i, result) in video_results.await.into_iter().enumerate() {
        if let Err(e) = result {
            error!("Video recording error for monitor {}: {:?}", i, e);
        }
    }
    if let Err(e) = audio_task.await {
        error!("Audio recording error: {:?}", e);
    }

    // Shutdown the whisper channel
    drop(whisper_sender_clone); // Close the sender channel

    // TODO: process any remaining audio chunks
    // TODO: wait a bit for whisper to finish processing
    // TODO: any additional cleanup like device controls to release

    info!("Stopped recording");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn record_video(
    db: Arc<DatabaseManager>,
    output_path: Arc<String>,
    fps: f64,
    is_running: Arc<AtomicBool>,
    ocr_engine: Arc<OcrEngine>,
    monitor_id: u32,
    use_pii_removal: bool,
    ignored_windows: Arc<[String]>,
    include_windows: Arc<[String]>,
    video_chunk_duration: Duration,
    languages: Arc<[Language]>,
    capture_unfocused_windows: bool,
    realtime_vision_sender: Arc<tokio::sync::broadcast::Sender<RealtimeVisionEvent>>,
) -> Result<()> {
    debug!("record_video: Starting");
    let db_chunk_callback = Arc::clone(&db);
    let rt = Handle::current();
    let device_name = Arc::new(format!("monitor_{}", monitor_id));

    let new_chunk_callback = {
        let db_chunk_callback = Arc::clone(&db_chunk_callback);
        let device_name = Arc::clone(&device_name);
        move |file_path: &str| {
            let file_path = file_path.to_string();
            let db_chunk_callback = Arc::clone(&db_chunk_callback);
            let device_name = Arc::clone(&device_name);
            rt.spawn(async move {
                if let Err(e) = db_chunk_callback
                    .insert_video_chunk(&file_path, &device_name)
                    .await
                {
                    error!("Failed to insert new video chunk: {}", e);
                }
                debug!("record_video: Inserted new video chunk: {}", file_path);
            });
        }
    };

    let video_capture = VideoCapture::new(
        &output_path,
        fps,
        video_chunk_duration,
        new_chunk_callback,
        Arc::downgrade(&ocr_engine),
        monitor_id,
        ignored_windows,
        include_windows,
        languages,
        capture_unfocused_windows,
    );

    while is_running.load(Ordering::SeqCst) {
        if let Some(frame) = video_capture.ocr_frame_queue.pop() {
            for window_result in &frame.window_ocr_results {
                match db.insert_frame(&device_name, None).await {
                    Ok(frame_id) => {
                        let text_json =
                            serde_json::to_string(&window_result.text_json).unwrap_or_default();

                        let text = if use_pii_removal {
                            &remove_pii(&window_result.text)
                        } else {
                            &window_result.text
                        };

                        let _ = realtime_vision_sender.send(RealtimeVisionEvent::Ocr(WindowOcr {
                            image: Some(frame.image.clone()),
                            text: text.clone(),
                            text_json: window_result.text_json.clone(),
                            app_name: window_result.app_name.clone(),
                            window_name: window_result.window_name.clone(),
                            focused: window_result.focused,
                            confidence: window_result.confidence,
                            timestamp: frame.timestamp,
                        }));
                        if let Err(e) = db
                            .insert_ocr_text(
                                frame_id,
                                text,
                                &text_json,
                                &window_result.app_name,
                                &window_result.window_name,
                                Arc::clone(&ocr_engine),
                                window_result.focused, // Add this line
                            )
                            .await
                        {
                            error!(
                                "Failed to insert OCR text: {}, skipping window {} of frame {}",
                                e, window_result.window_name, frame_id
                            );
                            continue;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to insert frame: {}", e);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_secs_f64(1.0 / fps)).await;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn record_audio(
    db: Arc<DatabaseManager>,
    chunk_duration: Duration,
    whisper_sender: crossbeam::channel::Sender<AudioInput>,
    whisper_receiver: crossbeam::channel::Receiver<TranscriptionResult>,
    audio_transcription_engine: Arc<AudioTranscriptionEngine>,
    realtime_audio_enabled: bool,
    devices: Arc<[DeviceType]>,
    languages: Arc<[Language]>,
    deepgram_api_key: Option<String>,
) -> Result<()> {
    let mut handles: HashMap<String, JoinHandle<()>> = HashMap::new();
    let mut previous_transcript = "".to_string();
    let mut previous_transcript_id: Option<i64> = None;
    loop {
        // Iterate over DashMap entries and process each device
        for device in devices.iter() {
            let device = match device {
                DeviceType::Audio(device) => device,
                _ => continue,
            };

            // Skip if we're already handling this device
            if handles.contains_key(&device.to_string()) {
                continue;
            }

            info!("Received audio device: {}", &device);
            let is_running = Arc::new(AtomicBool::new(true));

            if !is_running.load(Ordering::Relaxed) {
                info!("Device control signaled stop for device {}", &device);
                if let Some(handle) = handles.remove(&device.to_string()) {
                    handle.abort();
                    info!("Stopped thread for device {}", &device);
                }
                continue;
            }

            let whisper_sender_clone = whisper_sender.clone();

            let languages_clone = languages.clone();
            let deepgram_api_key_clone = deepgram_api_key.clone();
            let audio_device_clone = Arc::new(device.clone());
            let handle = tokio::spawn(async move {
                let audio_device_clone = Arc::clone(&audio_device_clone);
                let deepgram_api_key = deepgram_api_key_clone.clone();
                debug!(
                    "Starting audio capture thread for device: {}",
                    &audio_device_clone
                );

                let mut did_warn = false;
                let is_running = Arc::new(AtomicBool::new(true));

                while is_running.load(Ordering::Relaxed) {
                    let deepgram_api_key = deepgram_api_key.clone();
                    let is_running_loop = Arc::clone(&is_running); // Create separate reference for the loop
                    let audio_stream = match AudioStream::from_device(
                        audio_device_clone.clone(),
                        Arc::clone(&is_running_loop), // Clone from original Arc
                    )
                    .await
                    {
                        Ok(stream) => stream,
                        Err(e) => {
                            if e.to_string().contains("Audio device not found") {
                                if !did_warn {
                                    warn!("Audio device not found: {}", audio_device_clone.name);
                                    did_warn = true;
                                }
                                tokio::time::sleep(Duration::from_secs(1)).await;
                                continue;
                            } else {
                                error!("Failed to create audio stream: {}", e);
                                return;
                            }
                        }
                    };

                    let mut recording_handles: Vec<JoinHandle<()>> = vec![];

                    let audio_stream = Arc::new(audio_stream);
                    let whisper_sender_clone = whisper_sender_clone.clone();
                    let audio_stream_clone = audio_stream.clone();
                    let is_running_loop_clone = is_running_loop.clone();
                    let record_handle = Some(tokio::spawn(async move {
                        let _ = record_and_transcribe(
                            audio_stream,
                            chunk_duration,
                            whisper_sender_clone.clone(),
                            is_running_loop_clone.clone(),
                        )
                        .await;
                    }));

                    if let Some(handle) = record_handle {
                        recording_handles.push(handle);
                    }

                    let languages_clone = languages_clone.clone();
                    let is_running_loop = is_running_loop.clone();
                    let live_transcription_handle = Some(tokio::spawn(async move {
                        if realtime_audio_enabled {
                            let _ = start_realtime_recording(
                                audio_stream_clone,
                                languages_clone.clone(),
                                is_running_loop.clone(),
                                deepgram_api_key.clone(),
                            )
                            .await;
                        }
                    }));

                    if let Some(handle) = live_transcription_handle {
                        recording_handles.push(handle);
                    }

                    join_all(recording_handles).await;
                }

                info!(
                    "exiting audio capture thread for device: {}",
                    &audio_device_clone
                );
            });

            handles.insert(device.to_string(), handle);
        }

        handles.retain(|device_id, handle| {
            if handle.is_finished() {
                info!("Handle for device {} has finished", device_id);
                false
            } else {
                true
            }
        });

        while let Ok(mut transcription) = whisper_receiver.try_recv() {
            info!(
                "device {} received transcription {:?}",
                transcription.input.device, transcription.transcription
            );

            // Insert the new transcript after fetching
            let mut current_transcript: Option<String> = transcription.transcription.clone();
            let mut processed_previous: Option<String> = None;
            if let Some((previous, current)) =
                transcription.cleanup_overlap(previous_transcript.as_str())
            {
                if !previous.is_empty() && !current.is_empty() {
                    if previous != previous_transcript {
                        processed_previous = Some(previous);
                    }
                    if current_transcript.is_some()
                        && current != current_transcript.clone().unwrap_or_default()
                    {
                        current_transcript = Some(current);
                    }
                }
            }

            transcription.transcription = current_transcript.clone();
            if current_transcript.is_some() {
                previous_transcript = current_transcript.unwrap();
            } else {
                continue;
            }
            // Process the audio result
            match process_audio_result(
                &db,
                transcription,
                audio_transcription_engine.clone(),
                processed_previous,
                previous_transcript_id,
            )
            .await
            {
                Err(e) => error!("Error processing audio result: {}", e),
                Ok(id) => previous_transcript_id = id,
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn process_audio_result(
    db: &DatabaseManager,
    result: TranscriptionResult,
    audio_transcription_engine: Arc<AudioTranscriptionEngine>,
    previous_transcript: Option<String>,
    previous_transcript_id: Option<i64>,
) -> Result<Option<i64>, anyhow::Error> {
    if result.error.is_some() || result.transcription.is_none() {
        error!(
            "Error in audio recording: {}. Not inserting audio result",
            result.error.unwrap_or_default()
        );
        return Ok(None);
    }

    let speaker = get_or_create_speaker_from_embedding(db, &result.speaker_embedding).await?;

    info!("Detected speaker: {:?}", speaker);

    let transcription = result.transcription.unwrap();
    let transcription_engine = audio_transcription_engine.to_string();
    let mut chunk_id: Option<i64> = None;

    info!(
        "device {} inserting audio chunk: {:?}",
        result.input.device, result.path
    );
    if let Some(id) = previous_transcript_id {
        if let Some(prev_transcript) = previous_transcript {
            match db
                .update_audio_transcription(id, prev_transcript.as_str())
                .await
            {
                Ok(_) => {}
                Err(e) => error!(
                    "Failed to update transcription for {}: audio_chunk_id {}",
                    result.input.device, e
                ),
            }
        }
    }
    match db.get_or_insert_audio_chunk(&result.path).await {
        Ok(audio_chunk_id) => {
            if transcription.is_empty() {
                return Ok(Some(audio_chunk_id));
            }

            if let Err(e) = db
                .insert_audio_transcription(
                    audio_chunk_id,
                    &transcription,
                    0,
                    &transcription_engine,
                    &result.input.device,
                    Some(speaker.id),
                    Some(result.start_time),
                    Some(result.end_time),
                )
                .await
            {
                error!(
                    "Failed to insert audio transcription for device {}: {}",
                    result.input.device, e
                );
                return Ok(Some(audio_chunk_id));
            } else {
                debug!(
                    "Inserted audio transcription for chunk {} from device {} using {}",
                    audio_chunk_id, result.input.device, transcription_engine
                );
                chunk_id = Some(audio_chunk_id);
            }
        }
        Err(e) => error!(
            "Failed to insert audio chunk for device {}: {}",
            result.input.device, e
        ),
    }
    Ok(chunk_id)
}

async fn get_or_create_speaker_from_embedding(
    db: &DatabaseManager,
    embedding: &[f32],
) -> Result<Speaker, anyhow::Error> {
    let speaker = db.get_speaker_from_embedding(embedding).await?;
    if let Some(speaker) = speaker {
        Ok(speaker)
    } else {
        let speaker = db.insert_speaker(embedding).await?;
        Ok(speaker)
    }
}

pub async fn merge_speakers(
    db: &DatabaseManager,
    speaker_to_keep_id: i64,
    speaker_to_merge_id: i64,
) -> Result<Speaker, anyhow::Error> {
    // make sure both speakers exist
    let _ = db.get_speaker_by_id(speaker_to_keep_id).await?;
    let _ = db.get_speaker_by_id(speaker_to_merge_id).await?;

    // call merge method from db
    match db
        .merge_speakers(speaker_to_keep_id, speaker_to_merge_id)
        .await
    {
        Ok(speaker) => Ok(speaker),
        Err(e) => Err(anyhow::anyhow!("Failed to merge speakers: {}", e)),
    }
}
