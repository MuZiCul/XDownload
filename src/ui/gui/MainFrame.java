package ui.gui;

import downloader.YtDlpDownloader;
import ui.gui.panels.*;
import util.*;

import javax.swing.*;
import java.awt.*;

/**
 * 主窗口：标签页 + 状态栏
 */
public class MainFrame extends JFrame {

    public final YtDlpDownloader downloader;
    private final JTabbedPane tabbedPane;

    DownloadPanel downloadPanel;
    SettingsPanel settingsPanel;
    AboutPanel aboutPanel;

    public MainFrame() {
        super("XDownload v" + Version.CURRENT);
        I18n.loadSaved(); // 加载上次语言设置
        setDefaultCloseOperation(JFrame.EXIT_ON_CLOSE);
        setSize(960, 680);
        setLocationRelativeTo(null);

        // 单例下载器
        downloader = new YtDlpDownloader();

        // 标签页
        tabbedPane = new JTabbedPane();
        downloadPanel = new DownloadPanel(this);
        settingsPanel = new SettingsPanel(this);
        aboutPanel = new AboutPanel();

        tabbedPane.addTab(I18n.get("tab.download"), downloadPanel);
        tabbedPane.addTab(I18n.get("tab.settings"), settingsPanel);
        tabbedPane.addTab(I18n.get("tab.about"), aboutPanel);

        add(tabbedPane, BorderLayout.CENTER);

        // 底部占位
        add(new JLabel(" "), BorderLayout.SOUTH);

        // 首次运行？弹出引导对话框
        if (needSetupWizard()) {
            SwingUtilities.invokeLater(() -> {
                StartupWizard wizard = new StartupWizard(this);
                wizard.setVisible(true);
                refreshStatusBar();
                downloadPanel.applyCookiesToDownloader();
            });
        } else {
            // 已有配置，直接应用
            applySavedConfig();
        }
    }

    private boolean needSetupWizard() {
        if (!ProcessHelper.isYtDlpAvailable()) return true;
        java.util.Map<String, String> cfg = ConfigManager.load();
        return cfg.isEmpty() || (!cfg.containsKey("proxyHost") && !cfg.containsKey("cookiesFromBrowser"));
    }

    /** 应用已保存配置 */
    void applySavedConfig() {
        if (!ProxyConfig.isEnabled()) ConfigManager.applySavedProxy();
        String[] cookies = ConfigManager.loadSavedCookies();
        if (cookies[0] != null) downloader.setCookiesFromBrowser(cookies[0]);
        else if (cookies[1] != null) downloader.setCookiesFile(cookies[1]);
        downloadPanel.applyCookiesToDownloader();
        refreshStatusBar();
    }

    public void refreshStatusBar() {
        if (downloadPanel != null) downloadPanel.refreshStatusLines();
    }

    private String getYtDlpVersion() {
        try {
            ProcessHelper.CommandResult r = ProcessHelper.execute(
                    java.util.List.of(ProcessHelper.findYtDlp(), "--version"));
            if (r.isSuccess() && !r.stdout.isEmpty()) return r.stdout.get(0).trim();
        } catch (Exception ignored) {}
        return I18n.get("common.unknown");
    }
}
