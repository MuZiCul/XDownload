package ui.gui.workers;

import downloader.YtDlpDownloader;
import model.VideoInfo;
import ui.gui.panels.DownloadPanel;
import ui.gui.panels.LogPanel;
import util.NetworkDetect;
import util.ProxyConfig;

import javax.swing.*;
import java.io.IOException;
import java.util.List;

/** 后台获取视频信息 */
public class FetchInfoWorker extends SwingWorker<VideoInfo, String> {

    private final YtDlpDownloader downloader;
    private final String url;
    private final DownloadPanel panel;

    public FetchInfoWorker(YtDlpDownloader downloader, String url, DownloadPanel panel) {
        this.downloader = downloader;
        this.url = url;
        this.panel = panel;
    }

    @Override
    protected VideoInfo doInBackground() throws Exception {
        // URL 校验：仅允许 x.com 视频
        if (!url.toLowerCase().contains("x.com")) {
            throw new IOException("仅支持x视频链接");
        }

        // 网络预检：快速检测 x.com 可达性（5s 超时），避免 yt-dlp 等 30s
        LogPanel.log("[NET] Checking x.com reachability...");
        if (!NetworkDetect.isXAccessible()) {
            String tip = ProxyConfig.isEnabled()
                    ? "无法访问x，请检查代理是否正常"
                    : "无法访问x，请在设置中配置代理后重试";
            throw new IOException(tip);
        }

        LogPanel.log("[YT-DLP] Fetching video info: " + url);
        VideoInfo info = downloader.fetchVideoInfo(url);
        LogPanel.log("[OK] Got " + info.getFormats().size() + " formats");
        return info;
    }

    @Override
    protected void process(List<String> chunks) {}

    @Override
    protected void done() {
        try {
            VideoInfo info = get();
            panel.onVideoInfoReady(info);
        } catch (Exception e) {
            String msg = e.getMessage();
            if (e.getCause() != null) msg = e.getCause().getMessage();
            if (msg == null || msg.isEmpty()) msg = "获取视频信息失败";
            LogPanel.log("[ERROR] Fetch failed: " + msg);
            panel.onFetchError(msg);
        }
    }
}
