package ui.gui.panels;

import ui.gui.MainFrame;
import util.I18n;

import javax.swing.*;
import java.awt.*;

/** 设置标签页 */
public class SettingsPanel extends JPanel {

    public SettingsPanel(MainFrame mainFrame) {
        setLayout(new BoxLayout(this, BoxLayout.Y_AXIS));
        setBorder(BorderFactory.createEmptyBorder(8, 8, 8, 8));

        add(buildDownloadDirPanel());
        add(Box.createVerticalStrut(8));
        add(new ProxySettingsPanel(mainFrame));
        add(Box.createVerticalStrut(8));
        add(new CookiesSettingsPanel(mainFrame));
        add(Box.createVerticalStrut(8));
        add(buildToolsPanel(mainFrame));
        add(Box.createVerticalStrut(8));
        add(buildLanguagePanel(mainFrame));
        add(Box.createVerticalStrut(8));
        add(buildLogPanel());
        add(Box.createVerticalGlue());
    }

    private JPanel buildDownloadDirPanel() {
        JPanel p = new JPanel(new FlowLayout(FlowLayout.LEFT, 5, 5));
        p.setBorder(BorderFactory.createTitledBorder(I18n.get("settings.dir")));

        String saved = util.ConfigManager.loadDownloadDir();
        JTextField dirField = new JTextField(saved != null ? saved : "downloads", 25);
        p.add(dirField);

        JButton browseBtn = new JButton(I18n.get("opt.browse"));
        browseBtn.addActionListener(e -> {
            JFileChooser fc = new JFileChooser();
            fc.setFileSelectionMode(JFileChooser.DIRECTORIES_ONLY);
            fc.setCurrentDirectory(new java.io.File(dirField.getText()));
            if (fc.showOpenDialog(this) == JFileChooser.APPROVE_OPTION) {
                String path = fc.getSelectedFile().getAbsolutePath();
                dirField.setText(path);
                util.ConfigManager.saveDownloadDir(path);
            }
        });
        p.add(browseBtn);

        return p;
    }

    private JPanel buildLogPanel() {
        JPanel p = new JPanel(new FlowLayout(FlowLayout.LEFT, 10, 5));
        JButton viewLogBtn = new JButton("View Log");
        viewLogBtn.addActionListener(e -> {
            try {
                java.awt.Desktop.getDesktop().open(
                        util.AppHome.CONFIG.resolve("xdownload.log").toFile());
            } catch (Exception ex) {
                JOptionPane.showMessageDialog(this, "Cannot open log file: " + ex.getMessage());
            }
        });
        p.add(viewLogBtn);
        return p;
    }

    private JPanel buildToolsPanel(MainFrame mainFrame) {
        JPanel p = new JPanel();
        p.setLayout(new BoxLayout(p, BoxLayout.Y_AXIS));
        p.setBorder(BorderFactory.createTitledBorder("Tools"));

        JPanel btnRow = new JPanel(new FlowLayout(FlowLayout.LEFT, 6, 2));
        boolean hasYt = util.ProcessHelper.isYtDlpAvailable();
        JButton ytBtn = new JButton(hasYt ? "yt-dlp: Latest" : "Download yt-dlp");
        ytBtn.setEnabled(!hasYt);
        ytBtn.addActionListener(e -> {
            ytBtn.setEnabled(false); ytBtn.setText("yt-dlp: ...");
            new ui.gui.workers.BootstrapWorker(true, result -> {
                SwingUtilities.invokeLater(() -> {
                    boolean ok = result != null && !result.startsWith("failed");
                    ytBtn.setText(ok ? "yt-dlp: Latest" : "Download yt-dlp");
                    ytBtn.setEnabled(!ok);
                    mainFrame.refreshStatusBar();
                });
            }).execute();
        });
        btnRow.add(ytBtn);

        boolean hasFf = util.ProcessHelper.isFfmpegAvailable();
        JButton ffBtn = new JButton(hasFf ? "ffmpeg: Latest" : "Download ffmpeg");
        ffBtn.setEnabled(!hasFf);
        ffBtn.addActionListener(e -> {
            ffBtn.setEnabled(false); ffBtn.setText("ffmpeg: ...");
            new ui.gui.workers.BootstrapWorker(false, result -> {
                SwingUtilities.invokeLater(() -> {
                    boolean ok = result != null && !result.startsWith("failed");
                    ffBtn.setText(ok ? "ffmpeg: Latest" : "Download ffmpeg");
                    ffBtn.setEnabled(!ok);
                    mainFrame.refreshStatusBar();
                });
            }).execute();
        });
        btnRow.add(ffBtn);

        JLabel hint = new JLabel(I18n.get("tools.hint"));
        hint.setFont(hint.getFont().deriveFont(11f));
        hint.setForeground(Color.GRAY);

        p.add(btnRow);
        p.add(hint);
        return p;
    }

    private JPanel buildLanguagePanel(MainFrame mainFrame) {
        JPanel p = new JPanel(new FlowLayout(FlowLayout.LEFT, 10, 5));
        p.setBorder(BorderFactory.createTitledBorder(I18n.get("lang.title")));

        JComboBox<String> langCombo = new JComboBox<>(new String[]{
                I18n.get("lang.zh"), I18n.get("lang.en")});
        langCombo.setSelectedIndex("en".equals(I18n.getLang()) ? 1 : 0);

        JButton applyBtn = new JButton(I18n.get("cookies.save"));
        applyBtn.addActionListener(e -> {
            String sel = (String) langCombo.getSelectedItem();
            String code = sel.equals(I18n.get("lang.en")) ? "en" : "zh";
            I18n.setLang(code);
            JOptionPane.showMessageDialog(mainFrame,
                    I18n.get("lang.restart"),
                    I18n.get("lang.title"), JOptionPane.INFORMATION_MESSAGE);
        });

        p.add(langCombo);
        p.add(applyBtn);
        return p;
    }
}
