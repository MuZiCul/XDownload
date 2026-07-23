package ui.gui.panels;

import util.I18n;
import model.DownloadConfig;

import javax.swing.*;
import java.awt.*;
import java.io.File;

/** 下载选项面板：输出目录、音频提取、开始/取消按钮 */
public class DownloadOptionsPanel extends JPanel {

    private final DownloadPanel parent;
    private final JTextField dirField;
    private final JCheckBox audioCheck;
    private final JSpinner retriesSpinner;
    private final JButton startBtn;
    private final JButton cancelBtn;

    public DownloadOptionsPanel(DownloadPanel parent) {
        this.parent = parent;
        setBorder(BorderFactory.createTitledBorder(I18n.get("opt.title")));
        setLayout(new BoxLayout(this, BoxLayout.Y_AXIS));

        // 输出目录
        add(new JLabel(I18n.get("opt.dir")));
        JPanel dirPanel = new JPanel(new BorderLayout(4, 0));
        dirField = new JTextField("downloads");
        JButton browseBtn = new JButton(I18n.get("opt.browse"));
        browseBtn.addActionListener(e -> {
            JFileChooser fc = new JFileChooser();
            fc.setFileSelectionMode(JFileChooser.DIRECTORIES_ONLY);
            fc.setCurrentDirectory(new File(dirField.getText()));
            if (fc.showOpenDialog(this) == JFileChooser.APPROVE_OPTION) {
                dirField.setText(fc.getSelectedFile().getAbsolutePath());
            }
        });
        dirPanel.add(dirField, BorderLayout.CENTER);
        dirPanel.add(browseBtn, BorderLayout.EAST);
        add(dirPanel);
        add(Box.createVerticalStrut(8));

        // 音频提取
        audioCheck = new JCheckBox(I18n.get("opt.audio"));
        add(audioCheck);
        add(Box.createVerticalStrut(8));

        // 重试次数
        add(new JLabel(I18n.get("opt.retries")));
        retriesSpinner = new JSpinner(new SpinnerNumberModel(5, 0, 99, 1));
        retriesSpinner.setMaximumSize(new Dimension(80, 25));
        add(retriesSpinner);
        add(Box.createVerticalStrut(16));

        // 开始 / 取消按钮
        startBtn = new JButton(I18n.get("opt.start"));
        startBtn.setFont(startBtn.getFont().deriveFont(Font.BOLD, 16f));
        startBtn.setBackground(new Color(0, 150, 0));
        startBtn.setForeground(Color.WHITE);
        startBtn.addActionListener(e -> doStart());

        cancelBtn = new JButton(I18n.get("opt.cancel"));
        cancelBtn.setFont(cancelBtn.getFont().deriveFont(Font.BOLD, 16f));
        cancelBtn.setBackground(new Color(200, 0, 0));
        cancelBtn.setForeground(Color.WHITE);
        cancelBtn.setVisible(false);
        cancelBtn.addActionListener(e -> parent.cancelDownload());

        JPanel btnPanel = new JPanel(new GridLayout(2, 1, 0, 4));
        btnPanel.add(startBtn);
        btnPanel.add(cancelBtn);
        add(btnPanel);
        add(Box.createVerticalGlue());
    }

    private void doStart() {
        DownloadConfig config = new DownloadConfig();
        config.setOutputDir(dirField.getText().trim());
        config.setExtractAudio(audioCheck.isSelected());
        config.setRetries((Integer) retriesSpinner.getValue());
        parent.startDownload(config);
    }

    public void setDownloading(boolean downloading) {
        startBtn.setVisible(!downloading);
        cancelBtn.setVisible(downloading);
    }
}
