package ui.gui.workers;

import downloader.YtDlpDownloader;
import downloader.YtDlpDownloader.DownloadProgress;
import model.DownloadConfig;
import ui.gui.panels.DownloadPanel;

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
        return downloader.download(config, this::publish);
    }

    @Override
    protected void process(List<DownloadProgress> chunks) {
        for (DownloadProgress p : chunks) {
            panel.onDownloadProgress(p);
        }
    }

    @Override
    protected void done() {
        try {
            boolean success = get();
            String path = new File(config.getOutputDir()).getAbsolutePath();
            panel.onDownloadComplete(success, path);
        } catch (Exception e) {
            if (!isCancelled()) {
                panel.onDownloadComplete(false, "");
            }
        }
    }
}
