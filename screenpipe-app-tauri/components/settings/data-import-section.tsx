import React, { useState, useEffect } from "react";
import { Command, Trash2, Plus } from "lucide-react";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Command as TauriCommand } from "@tauri-apps/plugin-shell";
import { toast } from "../ui/use-toast";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { Card, CardContent } from "../ui/card";
import { Label } from "../ui/label";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "../ui/accordion";
import { writeTextFile, BaseDirectory, readDir } from "@tauri-apps/plugin-fs";
import { join } from "@tauri-apps/api/path";
import { appLocalDataDir } from "@tauri-apps/api/path";

interface VideoMetadata {
  file_path: string;
  metadata: {
    name?: string;
    creation_time?: string;
    fps?: number;
    device_name?: string;
  };
}

export function DataImportSection() {
  const [path, setPath] = useState("");
  const [isIndexing, setIsIndexing] = useState(false);
  const [detectedVideos, setDetectedVideos] = useState<string[]>([]);
  const [metadataConfig, setMetadataConfig] = useState<VideoMetadata[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [progress, setProgress] = useState<{
    current: number;
    total: number;
  } | null>(null);

  // Scan for videos in the selected path
  const scanForVideos = async () => {
    if (!path.trim()) return;

    try {
      console.log("starting video scan for path:", path);
      setIsScanning(true);

      // recursively read directory
      const entries = await readDir(path);
      console.log("found directory entries:", entries);

      if (entries.length > 10) {
        toast({
          title: "too many files",
          description: "please select a directory with 10 or fewer files",
          variant: "destructive",
        });
        return;
      }

      const videos = await Promise.all(entries.map((e) => join(path, e.name)));
      console.log("filtered video files:", videos);
      setDetectedVideos(videos);

      // initialize metadata config for each video
      const newMetadataConfig = videos.map((file_path) => ({
        file_path,
        metadata: {
          name: "",
          creation_time: new Date().toISOString(),
          fps: 30,
          device_name: "",
        },
      }));
      console.log("initialized metadata config:", newMetadataConfig);
      setMetadataConfig(newMetadataConfig);

      if (videos.length === 0) {
        toast({
          title: "no videos found",
          description:
            "no supported video files found in the selected directory",
          variant: "destructive",
        });
      } else {
        toast({
          title: "videos detected",
          description: `found ${videos.length} video files`,
        });
      }
    } catch (error: any) {
      console.error("scan error:", error);
      toast({
        title: "scanning failed",
        description: error.toString(),
        variant: "destructive",
      });
    } finally {
      setIsScanning(false);
    }
  };

  const handleMetadataChange = (
    index: number,
    field: string,
    value: string | number
  ) => {
    console.log("metadata change:", { index, field, value });
    setMetadataConfig((prev) => {
      const updated = [...prev];
      updated[index] = {
        ...updated[index],
        metadata: {
          ...updated[index].metadata,
          [field]: value,
        },
      };
      console.log("updated metadata config:", updated[index]);
      return updated;
    });
  };

  useEffect(() => {
    if (path.trim()) {
      scanForVideos();
    }
  }, [path]);

  const handleIndex = async () => {
    if (!path.trim()) return;

    try {
      console.log("starting indexing process for path:", path);
      setIsIndexing(true);
      setProgress(null);

      const configFileName = `metadata-override-${Date.now()}.json`;
      const configData = { overrides: metadataConfig };
      console.log("writing metadata config:", configData);

      await writeTextFile(configFileName, JSON.stringify(configData), {
        baseDir: BaseDirectory.AppLocalData,
      });

      const configPath = await join(await appLocalDataDir(), configFileName);
      console.log("config file path:", configPath);

      const command = TauriCommand.sidecar("screenpipe", [
        "add",
        path.trim(),
        "--metadata-override",
        configPath,
      ]);
      console.log("executing command:", command);

      command.stdout.on("data", (line) => {
        console.log("command output:", line);
        if (line.includes("found")) {
          const match = line.match(/found (\d+) video files/);
          if (match) {
            setProgress({ current: 0, total: parseInt(match[1]) });
          }
        }
        if (line.includes("processing video:")) {
          setProgress((prev) =>
            prev ? { ...prev, current: prev.current + 1 } : null
          );
        }
      });

      const output = await command.execute();
      console.log("command execution result:", output);

      if (output.code === 0) {
        toast({
          title: "data imported",
          description: "your data has been successfully imported",
        });
      } else {
        throw new Error(output.stderr);
      }
    } catch (error: any) {
      console.error("import error:", error);
      toast({
        title: "import failed",
        description: error.toString(),
        variant: "destructive",
      });
    } finally {
      setIsIndexing(false);
      setProgress(null);
    }
  };

  const handleSelectFolder = async () => {
    try {
      console.log("opening folder selector");
      const selected = await open({
        directory: true,
        multiple: false,
      });
      console.log("selected folder:", selected);

      if (selected && typeof selected === "string") {
        setPath(selected);
      }
    } catch (error: any) {
      console.error("folder selection error:", error);
      toast({
        title: "folder selection failed",
        description: error.toString(),
        variant: "destructive",
      });
    }
  };

  return (
    <div className="w-full space-y-6 py-4">
      <div>
        <h1 className="text-2xl font-bold">data import</h1>
        <p className="text-sm text-gray-500">
          add your own video recordings (mp4, mov, avi) into screenpipe
        </p>
      </div>

      <div className="space-y-4">
        <div className="flex gap-2">
          <Input
            placeholder="enter path to index (e.g., /path/to/files)"
            value={path}
            onChange={(e) => setPath(e.target.value)}
            className="font-mono"
          />
          <Button
            onClick={handleSelectFolder}
            variant="outline"
            className="whitespace-nowrap"
          >
            <FolderOpen className="h-4 w-4 mr-2" />
            select folder
          </Button>
        </div>

        {/* Metadata Configuration Section */}
        {detectedVideos.length > 0 && (
          <Card className="mt-4">
            <CardContent className="pt-6">
              <h3 className="text-lg font-semibold mb-4">
                configure video metadata
              </h3>
              <Accordion type="single" collapsible className="w-full">
                {detectedVideos.map((video, index) => (
                  <AccordionItem key={video} value={`video-${index}`}>
                    <AccordionTrigger className="text-sm">
                      {video.split("/").pop()}
                    </AccordionTrigger>
                    <AccordionContent>
                      <div className="space-y-4 p-4">
                        <div className="grid gap-4">
                          <div className="space-y-2">
                            <Label>custom name</Label>
                            <Input
                              placeholder="enter a custom name for this video"
                              value={metadataConfig[index]?.metadata.name || ""}
                              onChange={(e) =>
                                handleMetadataChange(
                                  index,
                                  "name",
                                  e.target.value
                                )
                              }
                            />
                          </div>
                          <div className="space-y-2">
                            <Label>device name</Label>
                            <Input
                              placeholder="enter device name"
                              value={
                                metadataConfig[index]?.metadata.device_name ||
                                ""
                              }
                              onChange={(e) =>
                                handleMetadataChange(
                                  index,
                                  "device_name",
                                  e.target.value
                                )
                              }
                            />
                          </div>
                          <div className="space-y-2">
                            <Label>fps</Label>
                            <Input
                              type="number"
                              placeholder="enter fps (optional)"
                              value={metadataConfig[index]?.metadata.fps || ""}
                              onChange={(e) =>
                                handleMetadataChange(
                                  index,
                                  "fps",
                                  parseFloat(e.target.value)
                                )
                              }
                            />
                          </div>
                          <div className="space-y-2">
                            <Label>creation time</Label>
                            <Input
                              type="datetime-local"
                              value={
                                metadataConfig[
                                  index
                                ]?.metadata.creation_time?.split("Z")[0] || ""
                              }
                              onChange={(e) =>
                                handleMetadataChange(
                                  index,
                                  "creation_time",
                                  new Date(e.target.value).toISOString()
                                )
                              }
                            />
                          </div>
                        </div>
                      </div>
                    </AccordionContent>
                  </AccordionItem>
                ))}
              </Accordion>
            </CardContent>
          </Card>
        )}

        {detectedVideos.length > 0 && (
          <div className="flex items-center gap-4">
            <Button onClick={handleIndex} disabled={isIndexing}>
              <Command className="h-4 w-4 mr-2" />
              {isIndexing ? "importing..." : "start import"}
            </Button>

            {/* Progress indicator */}
            {isIndexing && (
              <div className="flex-1 text-sm text-muted-foreground space-y-2">
                <div className="flex items-center gap-2">
                  <div className="animate-spin h-4 w-4 border-2 border-primary border-t-transparent rounded-full" />
                  <span>
                    {progress
                      ? `processing ${progress.current}/${progress.total} videos...`
                      : "analyzing files..."}
                  </span>
                </div>
                {progress && (
                  <div className="h-1 bg-secondary rounded-full overflow-hidden">
                    <div
                      className="h-full bg-primary transition-all duration-300"
                      style={{
                        width: `${(progress.current / progress.total) * 100}%`,
                      }}
                    />
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
