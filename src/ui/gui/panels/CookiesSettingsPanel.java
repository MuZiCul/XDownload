package ui.gui.panels;

import util.I18n;
import ui.gui.MainFrame;
import ui.gui.workers.CookiesValidationWorker;
import util.ConfigManager;
import util.ProcessHelper;

import javax.swing.*;
import java.awt.*;

/** Cookies 设置 */
public class CookiesSettingsPanel extends JPanel {

    private final MainFrame mainFrame;
    private final JComboBox<String> browserCombo;
    private final JButton validateBtn, saveBtn;
    private final JLabel statusLabel;

    private static final String[] BROWSERS = {I18n.get("cookies.none"), "chrome", "firefox", "edge", "brave", "opera"};

    public CookiesSettingsPanel(MainFrame mainFrame) {
        this.mainFrame = mainFrame;
        setBorder(BorderFactory.createTitledBorder("Cookies"));
        setLayout(new BoxLayout(this, BoxLayout.Y_AXIS));

        // 浏览器选择
        JPanel topPanel = new JPanel(new FlowLayout(FlowLayout.LEFT, 4, 0));
        browserCombo = new JComboBox<>(BROWSERS);
        String current = mainFrame.downloader.getCookiesFromBrowser();
        if (current != null) browserCombo.setSelectedItem(current);

        validateBtn = new JButton(I18n.get("cookies.validate"));
        validateBtn.addActionListener(e -> validateCookies());

        saveBtn = new JButton(I18n.get("cookies.save"));
        saveBtn.addActionListener(e -> saveCookies());

        topPanel.add(new JLabel(I18n.get("cookies.browser")));
        topPanel.add(browserCombo);
        topPanel.add(validateBtn);
        topPanel.add(saveBtn);
        add(topPanel);

        statusLabel = new JLabel(" ");
        add(statusLabel);
    }

    private void validateCookies() {
        String browser = (String) browserCombo.getSelectedItem();
        if (browser == null || "none".equals(browser)) return;
        statusLabel.setText("[...] " + I18n.get("cookies.validating") + " " + browser);
        new CookiesValidationWorker(browser, result -> {
            SwingUtilities.invokeLater(() -> statusLabel.setText(result.message));
        }).execute();
    }

    private void saveCookies() {
        String browser = (String) browserCombo.getSelectedItem();
        if (browser == null || "none".equals(browser)) {
            mainFrame.downloader.setCookiesFromBrowser(null);
            ConfigManager.clearCookies();
        } else {
            mainFrame.downloader.setCookiesFromBrowser(browser);
            ConfigManager.saveCookies(browser, null);
        }
        mainFrame.refreshStatusBar();
        statusLabel.setText("[+] " + I18n.get("cookies.saved") + browser);
    }
}
