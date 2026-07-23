package ui.gui;

import ui.gui.workers.*;
import util.*;

import javax.swing.*;
import java.awt.*;

public class StartupWizard extends JDialog {

    private final MainFrame mainFrame;
    private final CardLayout cards = new CardLayout();
    private final JPanel cardPanel = new JPanel(cards);

    private JLabel ytDlpStatus, ffmpegStatus;
    private JButton downloadToolsBtn, next1Btn;
    private JLabel netStatus, proxyTestResult;
    private JTextField proxyHost;
    private JSpinner proxyPort;
    private JButton testProxyBtn, next2Btn;
    private JComboBox<String> browserCombo;
    private JLabel cookiesStatus;
    private JButton validateCookiesBtn, finishBtn;

    public StartupWizard(MainFrame mainFrame) {
        super(mainFrame, I18n.get("wizard.title"), true);
        this.mainFrame = mainFrame;
        setSize(550, 440);
        setLocationRelativeTo(mainFrame);
        setDefaultCloseOperation(DO_NOTHING_ON_CLOSE);

        buildCard1();
        buildCard2();
        buildCard3();

        add(cardPanel);
        cards.show(cardPanel, "env");
    }

    private void buildCard1() {
        JPanel p = new JPanel(new BorderLayout(10, 10));
        p.setBorder(BorderFactory.createEmptyBorder(15, 15, 15, 15));
        p.add(new JLabel("<html><h2>" + I18n.get("wizard.env") + " (1/3)</h2></html>"), BorderLayout.NORTH);

        JPanel info = new JPanel();
        info.setLayout(new BoxLayout(info, BoxLayout.Y_AXIS));
        ytDlpStatus = new JLabel(I18n.get("wizard.ytdlp.checking"));
        ffmpegStatus = new JLabel(I18n.get("wizard.ffmpeg.checking"));
        info.add(ytDlpStatus);
        info.add(Box.createVerticalStrut(5));
        info.add(ffmpegStatus);
        p.add(info, BorderLayout.CENTER);

        JPanel btns = new JPanel(new FlowLayout(FlowLayout.RIGHT));
        downloadToolsBtn = new JButton(I18n.get("wizard.download.tools"));
        downloadToolsBtn.addActionListener(e -> downloadMissing());
        next1Btn = new JButton(I18n.get("wizard.next"));
        next1Btn.addActionListener(e -> cards.show(cardPanel, "proxy"));
        btns.add(downloadToolsBtn);
        btns.add(next1Btn);
        p.add(btns, BorderLayout.SOUTH);

        cardPanel.add(p, "env");
        checkTools();
    }

    private void checkTools() {
        ytDlpStatus.setText(I18n.get(ProcessHelper.isYtDlpAvailable() ? "wizard.ytdlp.ok" : "wizard.ytdlp.no"));
        ffmpegStatus.setText(I18n.get(ProcessHelper.isFfmpegAvailable() ? "wizard.ffmpeg.ok" : "wizard.ffmpeg.no"));
    }

    private void downloadMissing() {
        downloadToolsBtn.setEnabled(false);
        if (!ProcessHelper.isYtDlpAvailable()) {
            ytDlpStatus.setText("yt-dlp: " + I18n.get("prog.downloading"));
            new BootstrapWorker(true, result -> {
                SwingUtilities.invokeLater(() -> {
                    ytDlpStatus.setText("yt-dlp: " + (result != null && !result.startsWith("failed") ? I18n.get("status.ok") : "FAILED"));
                    checkFFmpeg();
                });
            }).execute();
        } else checkFFmpeg();
    }

    private void checkFFmpeg() {
        if (!ProcessHelper.isFfmpegAvailable()) {
            ffmpegStatus.setText("ffmpeg: " + I18n.get("prog.downloading"));
            new BootstrapWorker(false, result -> {
                SwingUtilities.invokeLater(() -> {
                    ffmpegStatus.setText("ffmpeg: " + (result != null && !result.startsWith("failed") ? I18n.get("status.ok") : "skipped"));
                    downloadToolsBtn.setEnabled(true);
                });
            }).execute();
        } else downloadToolsBtn.setEnabled(true);
    }

    private void buildCard2() {
        JPanel p = new JPanel(new BorderLayout(10, 10));
        p.setBorder(BorderFactory.createEmptyBorder(15, 15, 15, 15));
        p.add(new JLabel("<html><h2>" + I18n.get("wizard.proxy.title") + " (2/3)</h2></html>"), BorderLayout.NORTH);

        JPanel center = new JPanel();
        center.setLayout(new BoxLayout(center, BoxLayout.Y_AXIS));
        netStatus = new JLabel(I18n.get("proxy.detecting"));
        center.add(netStatus);

        JPanel input = new JPanel(new FlowLayout(FlowLayout.LEFT));
        input.add(new JLabel(I18n.get("proxy.host")));
        proxyHost = new JTextField("127.0.0.1", 12);
        input.add(proxyHost);
        input.add(new JLabel(I18n.get("proxy.port")));
        proxyPort = new JSpinner(new SpinnerNumberModel(7890, 1, 65535, 1));
        input.add(proxyPort);
        center.add(input);

        JPanel btns2 = new JPanel(new FlowLayout(FlowLayout.LEFT));
        testProxyBtn = new JButton(I18n.get("proxy.test"));
        testProxyBtn.addActionListener(e -> testProxy());
        btns2.add(testProxyBtn);
        proxyTestResult = new JLabel(" ");
        btns2.add(proxyTestResult);
        center.add(btns2);
        p.add(center, BorderLayout.CENTER);

        JPanel bottom = new JPanel(new FlowLayout(FlowLayout.RIGHT));
        JButton back2 = new JButton(I18n.get("wizard.back"));
        back2.addActionListener(e -> cards.show(cardPanel, "env"));
        next2Btn = new JButton(I18n.get("wizard.next"));
        next2Btn.addActionListener(e -> cards.show(cardPanel, "cookies"));
        JButton skip2 = new JButton(I18n.get("wizard.skip"));
        skip2.addActionListener(e -> cards.show(cardPanel, "cookies"));
        bottom.add(back2); bottom.add(next2Btn); bottom.add(skip2);
        p.add(bottom, BorderLayout.SOUTH);

        cardPanel.add(p, "proxy");

        new EnvironmentCheckWorker(overseas -> {
            SwingUtilities.invokeLater(() -> netStatus.setText(I18n.get(overseas ? "proxy.overseas" : "proxy.domestic")));
        }).execute();
    }

    private void testProxy() {
        String host = proxyHost.getText().trim();
        int port = (Integer) proxyPort.getValue();
        proxyTestResult.setText(I18n.get("proxy.testing"));
        new ProxyTestWorker(host, port, result -> {
            SwingUtilities.invokeLater(() -> {
                if (result != null && result.success) {
                    proxyTestResult.setText("[+] " + I18n.get("proxy.ok") + result.elapsedMs + "ms");
                    ProxyConfig.setProxy(host, port);
                    ConfigManager.saveProxy(host, port);
                    mainFrame.refreshStatusBar();
                } else proxyTestResult.setText("[-] " + I18n.get("proxy.failed"));
            });
        }).execute();
    }

    private void buildCard3() {
        JPanel p = new JPanel(new BorderLayout(10, 10));
        p.setBorder(BorderFactory.createEmptyBorder(15, 15, 15, 15));
        p.add(new JLabel("<html><h2>" + I18n.get("wizard.cookies.title") + " (3/3)</h2></html>"), BorderLayout.NORTH);

        JPanel center = new JPanel();
        center.setLayout(new BoxLayout(center, BoxLayout.Y_AXIS));

        JPanel selPanel = new JPanel(new FlowLayout(FlowLayout.LEFT));
        selPanel.add(new JLabel(I18n.get("cookies.browser")));
        browserCombo = new JComboBox<>(new String[]{"chrome", "firefox", "edge", "brave", "opera"});
        selPanel.add(browserCombo);
        center.add(selPanel);

        JPanel valPanel = new JPanel(new FlowLayout(FlowLayout.LEFT));
        validateCookiesBtn = new JButton(I18n.get("cookies.validate"));
        validateCookiesBtn.addActionListener(e -> validateCookies());
        cookiesStatus = new JLabel(" ");
        valPanel.add(validateCookiesBtn);
        valPanel.add(cookiesStatus);
        center.add(valPanel);
        p.add(center, BorderLayout.CENTER);

        JPanel bottom = new JPanel(new FlowLayout(FlowLayout.RIGHT));
        JButton back3 = new JButton(I18n.get("wizard.back"));
        back3.addActionListener(e -> cards.show(cardPanel, "proxy"));
        finishBtn = new JButton(I18n.get("wizard.finish"));
        finishBtn.addActionListener(e -> saveAndClose());
        bottom.add(back3); bottom.add(finishBtn);
        p.add(bottom, BorderLayout.SOUTH);

        cardPanel.add(p, "cookies");
        scanCookies();
    }

    private void scanCookies() {
        String[] browsers = {"chrome", "firefox", "edge", "brave", "opera"};
        cookiesStatus.setText(I18n.get("cookies.scanning"));
        for (String b : browsers) {
            ProcessHelper.CookiesValidationResult r = ProcessHelper.validateCookiesFromBrowser(b);
            if (r.success && r.cookieCount > 0) {
                browserCombo.setSelectedItem(b);
                cookiesStatus.setText("[+] " + b + ": " + r.cookieCount + " cookies");
                return;
            }
        }
        cookiesStatus.setText("[!] " + I18n.get("cookies.none"));
    }

    private void validateCookies() {
        String browser = (String) browserCombo.getSelectedItem();
        if (browser == null) return;
        cookiesStatus.setText(I18n.get("cookies.validating"));
        new CookiesValidationWorker(browser, result -> {
            SwingUtilities.invokeLater(() -> cookiesStatus.setText(result != null ? result.message : "[-] Failed"));
        }).execute();
    }

    private void saveAndClose() {
        String browser = (String) browserCombo.getSelectedItem();
        if (browser != null) {
            mainFrame.downloader.setCookiesFromBrowser(browser);
            ConfigManager.saveCookies(browser, null);
        }
        mainFrame.applySavedConfig();
        mainFrame.refreshStatusBar();
        dispose();
    }
}
