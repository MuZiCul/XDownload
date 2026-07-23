package ui.gui.workers;

import downloader.YtDlpDownloader;
import downloader.YtDlpDownloader.DownloadProgress;
import model.DownloadConfig;
import ui.gui.panels.DownloadPanel;
import ui.gui.panels.LogPanel;

import javax.swing.*;
import java.io.File;
import java.util.List;

/** 后台下载视频 */
public class DownloadWorker extends SwingWorker<Boolean, DownloadProgress> {

    private final YtDlpDownloader downloader;
    private final DownloadConfig config;
    private final DownloadPanel panel;

    public DownloadWorker(YtDlpDownloader downloader, DownloadConfig config, DownloadPanel panel) {
        this.downloader = downloader;
        this.config = config;
        this.panel = panel;
    }

    @Override
    protected Boolean doInBackground() throws Exception {
        LogPanel.log("[YT-DLP] Starting download...");
        boolean result = downloader.download(config, progress -> {
            publish(progress);
            if (progress.status != null && !"downloading".equals(progress.status)) {
                LogPanel.log("[YT-DLP] " + progress.status + " | " + progress.percent);
            }
        });
        LogPanel.log(result ? "[OK] Download complete" : "[FAIL] Download failed");
        return result;
    }

    @Override
    protected void process(List<DownloadProgress> chunks) {
        for (DownloadProgress p : chunks) panel.onDownloadProgress(p);
    }

    @Override
    protected void done() {
        try {
            boolean success = get();
            String path = new File(config.getOutputDir()).getAbsolutePath();
            panel.onDownloadComplete(success, path);
        } catch (Exception e) {
            if (!isCancelled()) {
                LogPanel.log("[ERROR] Download exception: " + e.getMessage());
                panel.onDownloadComplete(false, "");
            }
        }
    }
}
