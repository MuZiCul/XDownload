package ui.gui.panels;

import util.I18n;
import ui.gui.MainFrame;
import ui.gui.workers.UpdateWorker;
import util.ProcessHelper;
import util.Version;

import javax.swing.*;
import java.awt.*;

/** 关于标签页 */
public class AboutPanel extends JPanel {

    private final MainFrame mainFrame;
    private final JLabel versionLabel, ytDlpLabel, ffmpegLabel;

    public AboutPanel(MainFrame mainFrame) {
        this.mainFrame = mainFrame;
        setLayout(new BoxLayout(this, BoxLayout.Y_AXIS));
        setBorder(BorderFactory.createEmptyBorder(30, 30, 30, 30));

        JLabel title = new JLabel("XDownload");
        title.setFont(title.getFont().deriveFont(Font.BOLD, 24f));
        title.setAlignmentX(CENTER_ALIGNMENT);
        add(title);

        versionLabel = new JLabel("v" + Version.CURRENT);
        versionLabel.setAlignmentX(CENTER_ALIGNMENT);
        add(versionLabel);

        add(Box.createVerticalStrut(20));

        JLabel desc = new JLabel(I18n.get("about.desc"));
        desc.setAlignmentX(CENTER_ALIGNMENT);
        add(desc);

        JLabel author = new JLabel("By MuZiCul");
        author.setAlignmentX(CENTER_ALIGNMENT);
        add(author);

        add(Box.createVerticalStrut(20));

        ytDlpLabel = new JLabel("yt-dlp: " + getYtDlpVer());
        ytDlpLabel.setAlignmentX(CENTER_ALIGNMENT);
        add(ytDlpLabel);

        ffmpegLabel = new JLabel(I18n.get("about.ffmpeg") + (ProcessHelper.isFfmpegAvailable() ? I18n.get("about.ffmpeg.ok") : I18n.get("about.ffmpeg.no")));
        ffmpegLabel.setAlignmentX(CENTER_ALIGNMENT);
        add(ffmpegLabel);

        add(Box.createVerticalStrut(20));

        JButton updateBtn = new JButton(I18n.get("about.update"));
        updateBtn.setAlignmentX(CENTER_ALIGNMENT);
        updateBtn.addActionListener(e -> updateYtDlp());
        add(updateBtn);
    }

    private String getYtDlpVer() {
        try {
            ProcessHelper.CommandResult r = ProcessHelper.execute(
                    java.util.List.of(ProcessHelper.findYtDlp(), "--version"));
            return r.isSuccess() && !r.stdout.isEmpty() ? r.stdout.get(0).trim() : "unknown";
        } catch (Exception e) { return "unknown"; }
    }

    private void updateYtDlp() {
        JDialog dialog = new JDialog((Frame) SwingUtilities.getWindowAncestor(this), I18n.get("about.update.title"), true);
        dialog.setSize(400, 100);
        dialog.setLocationRelativeTo(this);
        JProgressBar bar = new JProgressBar();
        bar.setIndeterminate(true);
        JLabel label = new JLabel(I18n.get("about.updating"));
        JPanel panel = new JPanel(new BorderLayout(5, 5));
        panel.setBorder(BorderFactory.createEmptyBorder(10, 10, 10, 10));
        panel.add(label, BorderLayout.NORTH);
        panel.add(bar, BorderLayout.CENTER);
        dialog.add(panel);

        UpdateWorker worker = new UpdateWorker(mainFrame.downloader, success -> {
            dialog.dispose();
            ytDlpLabel.setText("yt-dlp: " + getYtDlpVer());
            mainFrame.refreshStatusBar();
        });
        worker.execute();
        dialog.setVisible(true);
    }
}
