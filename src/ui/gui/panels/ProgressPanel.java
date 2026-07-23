package ui.gui.panels;

import downloader.YtDlpDownloader.DownloadProgress;
import util.I18n;

import javax.swing.*;
import java.awt.*;

public class ProgressPanel extends JPanel {

    private final JLabel statusLabel;
    private long startTime;

    public ProgressPanel() {
        super(new FlowLayout(FlowLayout.LEFT, 10, 2));
        setBorder(BorderFactory.createEmptyBorder(4, 0, 0, 0));

        statusLabel = new JLabel(" ");
        statusLabel.setFont(statusLabel.getFont().deriveFont(Font.BOLD, 13f));
        add(statusLabel);
    }

    public void reset() {
        statusLabel.setText(I18n.get("prog.downloading"));
        startTime = System.currentTimeMillis();
    }

    public void setStatus(String text) { statusLabel.setText(text); }

    public void setError(String text) { statusLabel.setText(text); }

    public void setComplete() {
        long elapsed = (System.currentTimeMillis() - startTime) / 1000;
        statusLabel.setText("[+] " + I18n.get("prog.complete") + " (" + elapsed + I18n.get("common.seconds") + ")");
    }

    public void setIndeterminate(boolean b) {} // no-op, no progress bar

    public void updateProgress(DownloadProgress p) {
        String eta = p.eta != null && !p.eta.isEmpty() && !"NA".equals(p.eta) ? " ETA:" + p.eta : "";
        statusLabel.setText(p.percent + " | " + (p.speed != null ? p.speed : "?") + eta);
    }
}
