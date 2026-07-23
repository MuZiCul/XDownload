package model;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

/**
 * 视频信息模型，存储从 yt-dlp 解析出的视频元数据
 */
public class VideoInfo {
    private final String url;
    private String title;
    private String description;
    private long duration;          // 秒
    private String thumbnailUrl;
    private String uploader;
    private long viewCount;
    private long likeCount;
    private final List<Format> formats;

    public VideoInfo(String url) {
        this.url = url;
        this.formats = new ArrayList<>();
    }

    // --- Getters / Setters ---

    public String getUrl() { return url; }

    public String getTitle() { return title; }
    public void setTitle(String title) { this.title = title; }

    public String getDescription() { return description; }
    public void setDescription(String description) { this.description = description; }

    public long getDuration() { return duration; }
    public void setDuration(long duration) { this.duration = duration; }

    public String getThumbnailUrl() { return thumbnailUrl; }
    public void setThumbnailUrl(String thumbnailUrl) { this.thumbnailUrl = thumbnailUrl; }

    public String getUploader() { return uploader; }
    public void setUploader(String uploader) { this.uploader = uploader; }

    public long getViewCount() { return viewCount; }
    public void setViewCount(long viewCount) { this.viewCount = viewCount; }

    public long getLikeCount() { return likeCount; }
    public void setLikeCount(long likeCount) { this.likeCount = likeCount; }

    public List<Format> getFormats() { return formats; }
    public void addFormat(Format format) { this.formats.add(format); }

    /**
     * 获取最佳视频+音频合并格式
     */
    public Format getBestFormat() {
        Format best = null;
        int bestScore = -1;
        for (Format f : formats) {
            int score = 0;
            if (f.hasVideo()) score += f.getHeight() != null ? f.getHeight() : 0;
            if (f.hasAudio()) score += 1000;
            if (score > bestScore) {
                bestScore = score;
                best = f;
            }
        }
        return best;
    }

    /**
     * 获取指定分辨率的格式
     */
    public Format getFormatByHeight(int height) {
        return formats.stream()
                .filter(f -> f.hasVideo() && f.hasAudio()
                        && f.getHeight() != null && f.getHeight() <= height)
                .max((a, b) -> {
                    int ha = a.getHeight() != null ? a.getHeight() : 0;
                    int hb = b.getHeight() != null ? b.getHeight() : 0;
                    return Integer.compare(ha, hb);
                })
                .orElse(null);
    }

    @Override
    public String toString() {
        StringBuilder sb = new StringBuilder();
        sb.append("===========================================\n");
        sb.append("  标题: ").append(title != null ? title : "未知").append("\n");
        sb.append("  作者: ").append(uploader != null ? uploader : "未知").append("\n");
        sb.append("  时长: ").append(formatDuration(duration)).append("\n");
        sb.append("  播放: ").append(formatNumber(viewCount)).append("\n");
        sb.append("===========================================\n");
        sb.append("  可用格式:\n");
        for (int i = 0; i < formats.size(); i++) {
            sb.append(String.format("    [%2d] %s\n", i, formats.get(i)));
        }
        return sb.toString();
    }

    // --- 静态工具方法 ---

    public static String formatDuration(long seconds) {
        if (seconds <= 0) return "未知";
        long h = seconds / 3600;
        long m = (seconds % 3600) / 60;
        long s = seconds % 60;
        if (h > 0) return String.format("%d:%02d:%02d", h, m, s);
        return String.format("%d:%02d", m, s);
    }

    public static String formatNumber(long n) {
        if (n >= 1_0000_0000) return String.format("%.1f亿", n / 1_0000_0000.0);
        if (n >= 1_0000) return String.format("%.1f万", n / 1_0000.0);
        return String.valueOf(n);
    }

    /**
     * 视频格式子类
     */
    public static class Format {
        private final String formatId;
        private String extension;       // mp4, webm, mkv ...
        private String resolution;      // 1920x1080
        private Integer height;         // 1080
        private Integer width;          // 1920
        private long fileSize;          // bytes, 0 = unknown
        private float fps;              // 帧率
        private String videoCodec;      // avc1, vp9 ...
        private String audioCodec;      // mp4a, opus ...
        private boolean hasVideo;
        private boolean hasAudio;
        private String note;            // 备注，如 "Premium"

        public Format(String formatId) {
            this.formatId = formatId;
        }

        public boolean hasVideo() { return hasVideo; }
        public boolean hasAudio() { return hasAudio; }
        public Integer getHeight() { return height; }

        public String getFormatId() { return formatId; }
        public void setFormatId(String formatId) { /* no-op, use constructor */ }
        public String getExtension() { return extension; }
        public void setExtension(String extension) { this.extension = extension; }
        public String getResolution() { return resolution; }
        public void setResolution(String resolution) { this.resolution = resolution; }
        public Integer getWidth() { return width; }
        public void setWidth(Integer width) { this.width = width; }
        public void setHeight(Integer height) { this.height = height; }
        public long getFileSize() { return fileSize; }
        public void setFileSize(long fileSize) { this.fileSize = fileSize; }
        public float getFps() { return fps; }
        public void setFps(float fps) { this.fps = fps; }
        public String getVideoCodec() { return videoCodec; }
        public void setVideoCodec(String videoCodec) { this.videoCodec = videoCodec; }
        public String getAudioCodec() { return audioCodec; }
        public void setAudioCodec(String audioCodec) { this.audioCodec = audioCodec; }
        public void setHasVideo(boolean hasVideo) { this.hasVideo = hasVideo; }
        public void setHasAudio(boolean hasAudio) { this.hasAudio = hasAudio; }
        public String getNote() { return note; }
        public void setNote(String note) { this.note = note; }

        @Override
        public String toString() {
            StringBuilder sb = new StringBuilder();
            // format_id: 截断过长的 ID
            String shortId = formatId.length() > 22 ? formatId.substring(0, 19) + "..." : formatId;
            sb.append(String.format("%-24s", shortId));
            // 扩展名
            sb.append(String.format("%-6s", extension != null ? extension : "?"));
            // 分辨率 / 类型标注
            String resLabel = resolution;
            if (resLabel == null && !hasVideo) resLabel = "audio only";
            if (resLabel != null) sb.append(String.format("%-12s", resLabel));
            else sb.append("            ");
            // 文件大小
            if (fileSize > 0) sb.append(formatFileSize(fileSize));
            else sb.append("?        ");
            // fps
            if (fps > 0) sb.append(String.format(" %.0ffps", fps));
            // 类型标签
            if (hasVideo && hasAudio) sb.append("  [V][A]");
            else if (hasVideo) sb.append("  [V]");
            else if (hasAudio) sb.append("  [A]");

            return sb.toString();
        }
    }

    public static String formatFileSize(long bytes) {
        if (bytes < 1024) return bytes + " B";
        if (bytes < 1024 * 1024) return String.format("%.1f KB", bytes / 1024.0);
        if (bytes < 1024 * 1024 * 1024) return String.format("%.1f MB", bytes / (1024.0 * 1024));
        return String.format("%.2f GB", bytes / (1024.0 * 1024 * 1024));
    }
}
