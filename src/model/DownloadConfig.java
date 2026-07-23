package model;

import java.io.File;

/**
 * 下载配置
 */
public class DownloadConfig {
    private String url;
    private String formatId = "best";         // 格式ID，默认最佳
    private String outputDir = "downloads";    // 输出目录
    private String outputTemplate = "%(title)s.%(ext)s";  // 文件名模板
    private boolean extractAudio = false;      // 仅提取音频
    private boolean embedSubtitles = false;    // 嵌入字幕
    private boolean embedThumbnail = false;    // 嵌入缩略图
    private boolean writeThumbnail = false;    // 下载缩略图
    private String proxy;                      // 代理地址
    private int retries = 5;                   // 重试次数
    private int socketTimeout = 30;            // 超时（秒）
    private String cookiesFile;                // cookies文件 (Netscape格式)
    private String cookiesFromBrowser;         // 从浏览器读取cookies (chrome/firefox/edge/brave/opera)
    private int maxHeight = 0;                 // 最大分辨率，0=不限
    private String downloadArchive;            // 下载归档文件，用于去重

    public DownloadConfig() {}

    public DownloadConfig(String url) {
        this.url = url;
    }

    // --- Getters / Setters ---

    public String getUrl() { return url; }
    public void setUrl(String url) { this.url = url; }

    public String getFormatId() { return formatId; }
    public void setFormatId(String formatId) { this.formatId = formatId; }

    public String getOutputDir() { return outputDir; }
    public void setOutputDir(String outputDir) { this.outputDir = outputDir; }

    public String getOutputTemplate() { return outputTemplate; }
    public void setOutputTemplate(String outputTemplate) { this.outputTemplate = outputTemplate; }

    public boolean isExtractAudio() { return extractAudio; }
    public void setExtractAudio(boolean extractAudio) { this.extractAudio = extractAudio; }

    public boolean isEmbedSubtitles() { return embedSubtitles; }
    public void setEmbedSubtitles(boolean embedSubtitles) { this.embedSubtitles = embedSubtitles; }

    public boolean isEmbedThumbnail() { return embedThumbnail; }
    public void setEmbedThumbnail(boolean embedThumbnail) { this.embedThumbnail = embedThumbnail; }

    public boolean isWriteThumbnail() { return writeThumbnail; }
    public void setWriteThumbnail(boolean writeThumbnail) { this.writeThumbnail = writeThumbnail; }

    public String getProxy() { return proxy; }
    public void setProxy(String proxy) { this.proxy = proxy; }

    public int getRetries() { return retries; }
    public void setRetries(int retries) { this.retries = retries; }

    public int getSocketTimeout() { return socketTimeout; }
    public void setSocketTimeout(int socketTimeout) { this.socketTimeout = socketTimeout; }

    public String getCookiesFile() { return cookiesFile; }
    public void setCookiesFile(String cookiesFile) { this.cookiesFile = cookiesFile; }

    public String getCookiesFromBrowser() { return cookiesFromBrowser; }
    public void setCookiesFromBrowser(String cookiesFromBrowser) { this.cookiesFromBrowser = cookiesFromBrowser; }

    public int getMaxHeight() { return maxHeight; }
    public void setMaxHeight(int maxHeight) { this.maxHeight = maxHeight; }

    public String getDownloadArchive() { return downloadArchive; }
    public void setDownloadArchive(String downloadArchive) { this.downloadArchive = downloadArchive; }

    /**
     * 获取完整输出路径模板
     */
    public String getOutputPath() {
        String dir = outputDir;
        if (!dir.endsWith(File.separator)) dir += File.separator;
        return dir + outputTemplate;
    }

    @Override
    public String toString() {
        return "DownloadConfig{" +
                "url='" + url + '\'' +
                ", formatId='" + formatId + '\'' +
                ", outputDir='" + outputDir + '\'' +
                ", extractAudio=" + extractAudio +
                '}';
    }
}
