## Bug Fixes
- Fixed silent screen recording data loss: FFmpeg crashes now auto-recover instead of silently destroying frames forever
- Fixed video chunk DB registration race condition that could cause hours of missing timeline data
- Pipeline now self-heals after 100 consecutive database errors instead of looping silently
- Dead capture/OCR/video tasks now trigger automatic restart instead of running as empty shells
- Frame drops and queue overflows are now visible in logs (previously hidden at debug level)

#### **Full Changelog:** [300d588c..68973c9d](https://github.com/mediar-ai/screenpipe/compare/300d588c..68973c9d)
