package ui.gui.panels;

import util.I18n;
import ui.gui.MainFrame;
import ui.gui.workers.EnvironmentCheckWorker;
import ui.gui.workers.ProxyTestWorker;
import util.ConfigManager;
import util.NetworkDetect;
import util.ProxyConfig;

import javax.swing.*;
import java.awt.*;

/** 代理设置 */
public class ProxySettingsPanel extends JPanel {

    private final MainFrame mainFrame;
    private final JRadioButton noneRadio, manualRadio;
    private final JTextField hostField;
    private final JSpinner portSpinner;
    private final JButton testBtn;
    private final JLabel statusLabel;

    public ProxySettingsPanel(MainFrame mainFrame) {
        this.mainFrame = mainFrame;
        setBorder(BorderFactory.createTitledBorder(I18n.get("proxy.title")));
        setLayout(new BoxLayout(this, BoxLayout.Y_AXIS));

        // 单选按钮组
        noneRadio = new JRadioButton(I18n.get("proxy.none"));
        manualRadio = new JRadioButton(I18n.get("proxy.manual"));
        ButtonGroup group = new ButtonGroup();
        group.add(noneRadio);
        group.add(manualRadio);

        // 初始化状态
        if (ProxyConfig.isEnabled()) {
            manualRadio.setSelected(true);
        } else {
            noneRadio.setSelected(true);
        }

        noneRadio.addActionListener(e -> { disableProxy(); });
        manualRadio.addActionListener(e -> { enableFields(true); });

        add(noneRadio);
        add(manualRadio);

        // 手动代理输入
        JPanel inputPanel = new JPanel(new FlowLayout(FlowLayout.LEFT, 4, 0));
        hostField = new JTextField(ProxyConfig.getProxyHost() != null ? ProxyConfig.getProxyHost() : "127.0.0.1", 12);
        portSpinner = new JSpinner(new SpinnerNumberModel(
                ProxyConfig.getProxyPort() > 0 ? ProxyConfig.getProxyPort() : 7890, 1, 65535, 1));
        inputPanel.add(new JLabel(I18n.get("proxy.host")));
        inputPanel.add(hostField);
        inputPanel.add(new JLabel(I18n.get("proxy.port")));
        inputPanel.add(portSpinner);
        add(inputPanel);

        // 测试按钮
        JPanel btnPanel = new JPanel(new FlowLayout(FlowLayout.LEFT, 4, 0));
        testBtn = new JButton(I18n.get("proxy.test"));
        testBtn.addActionListener(e -> testProxy());
        btnPanel.add(testBtn);

        JButton detectBtn = new JButton(I18n.get("proxy.autodetect"));
        detectBtn.addActionListener(e -> autoDetect());
        btnPanel.add(detectBtn);
        add(btnPanel);

        // 状态
        statusLabel = new JLabel(" ");
        add(statusLabel);

        enableFields(ProxyConfig.isEnabled());
    }

    private void enableFields(boolean enabled) {
        hostField.setEnabled(enabled);
        portSpinner.setEnabled(enabled);
        testBtn.setEnabled(enabled);
    }

    private void testProxy() {
        String host = hostField.getText().trim();
        int port = (Integer) portSpinner.getValue();
        statusLabel.setText(I18n.get("proxy.testing"));
        ProxyConfig.setProxy(host, port);
        new ProxyTestWorker(host, port, result -> {
            SwingUtilities.invokeLater(() -> {
                statusLabel.setText(result.success
                        ? I18n.get("proxy.ok") + result.elapsedMs + "ms"
                        : "[-] " + result.message + " (" + result.elapsedMs + "ms)");
                if (result.success) {
                    ConfigManager.saveProxy(host, port);
                    mainFrame.refreshStatusBar();
                }
            });
        }).execute();
    }

    private void autoDetect() {
        statusLabel.setText(I18n.get("proxy.detecting"));
        new EnvironmentCheckWorker(overseas -> {
            SwingUtilities.invokeLater(() -> {
                if (overseas) {
                    statusLabel.setText(I18n.get("proxy.overseas"));
                    noneRadio.setSelected(true);
                    disableProxy();
                } else {
                    statusLabel.setText(I18n.get("proxy.domestic"));
                    manualRadio.setSelected(true);
                    enableFields(true);
                }
            });
        }).execute();
    }

    private void disableProxy() {
        ProxyConfig.disable();
        ConfigManager.removeProxy();
        enableFields(false);
        mainFrame.refreshStatusBar();
        statusLabel.setText(I18n.get("proxy.disabled"));
    }
}
