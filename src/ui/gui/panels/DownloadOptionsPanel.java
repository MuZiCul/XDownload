package ui.gui.panels;

import model.DownloadConfig;

import javax.swing.*;

/** 下载选项：被 FormatTablePanel 和 DownloadPanel 调用的逻辑 */
public class DownloadOptionsPanel {

    private final DownloadPanel parent;
    private int retries = 5;

    public DownloadOptionsPanel(DownloadPanel parent) {
        this.parent = parent;
    }

    public void setRetries(int r) { this.retries = r; }

    public void doStart() {
        DownloadConfig config = parent.getCurrentConfig();
        if (config == null || config.getUrl() == null) {
            JOptionPane.showMessageDialog(null, "Please fetch video info first.", "No URL", JOptionPane.WARNING_MESSAGE);
            return;
        }
        config.setRetries(retries);
        String saved = util.ConfigManager.loadDownloadDir();
        config.setOutputDir(saved != null && !saved.isEmpty() ? saved : "downloads");
        parent.startDownload(config);
    }
}
