package ui.gui.panels;

import util.I18n;
import downloader.YtDlpDownloader.DownloadProgress;

import javax.swing.*;
import java.awt.*;

/** 下载进度条 + 速度/ETA/状态 */
public class ProgressPanel extends JPanel {

    private final JProgressBar progressBar;
    private final JLabel speedLabel, etaLabel, statusLabel;
    private long startTime;

    public ProgressPanel() {
        super(new BorderLayout(5, 3));
        setBorder(BorderFactory.createEmptyBorder(4, 0, 0, 0));

        progressBar = new JProgressBar(0, 100);
        progressBar.setStringPainted(true);
        add(progressBar, BorderLayout.NORTH);

        JPanel infoPanel = new JPanel(new FlowLayout(FlowLayout.LEFT, 15, 0));
        speedLabel = new JLabel(" ");
        etaLabel = new JLabel(" ");
        statusLabel = new JLabel(" ");
        infoPanel.add(speedLabel);
        infoPanel.add(etaLabel);
        infoPanel.add(statusLabel);
        add(infoPanel, BorderLayout.CENTER);
    }

    public void reset() {
        progressBar.setValue(0);
        progressBar.setIndeterminate(false);
        speedLabel.setText(" ");
        etaLabel.setText(" ");
        statusLabel.setText(" ");
        startTime = System.currentTimeMillis();
    }

    public void setIndeterminate(boolean indeterminate) {
        progressBar.setIndeterminate(indeterminate);
    }

    public void setStatus(String text) {
        statusLabel.setText(text);
    }

    public void setError(String text) {
        progressBar.setValue(0);
        statusLabel.setText(text);
    }

    public void setComplete() {
        progressBar.setValue(100);
        long elapsed = (System.currentTimeMillis() - startTime) / 1000;
        statusLabel.setText("[+] " + I18n.get("prog.complete") + " (" + elapsed + I18n.get("common.seconds") + ")");
    }

    public void updateProgress(DownloadProgress p) {
        double pct = p.getPercentValue();
        progressBar.setValue((int) pct);
        progressBar.setIndeterminate(false);
        if (p.speed != null && !p.speed.isEmpty() && !"NA".equals(p.speed)) {
            speedLabel.setText(p.speed);
        }
        if (p.eta != null && !p.eta.isEmpty() && !"NA".equals(p.eta)) {
            etaLabel.setText(I18n.get("prog.eta") + p.eta);
        }
        if (p.status != null && !p.status.isEmpty()) {
            statusLabel.setText(p.status);
        }
    }
}
