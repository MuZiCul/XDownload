package ui.gui.workers;

import downloader.YtDlpDownloader;
import model.VideoInfo;
import ui.gui.panels.DownloadPanel;
import ui.gui.panels.LogPanel;

import javax.swing.*;
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
            LogPanel.log("[ERROR] Fetch failed: " + msg);
            panel.onFetchError(msg != null ? msg : "Unknown error");
        }
    }
}
