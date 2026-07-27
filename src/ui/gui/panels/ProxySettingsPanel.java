package ui.gui.panels;

import util.I18n;
import ui.gui.MainFrame;
import ui.gui.workers.EnvironmentCheckWorker;
import ui.gui.workers.ProxyTestWorker;
import util.ConfigManager;
import util.ProxyConfig;

import javax.swing.*;
import java.awt.*;

public class ProxySettingsPanel extends JPanel {

    private final MainFrame mainFrame;
    private final JRadioButton noneRadio, manualRadio;
    private final JTextField hostField;
    private final JSpinner portSpinner;
    private final JButton testBtn, detectBtn;
    private final JLabel statusLabel;

    public ProxySettingsPanel(MainFrame mainFrame) {
        this.mainFrame = mainFrame;
        setBorder(BorderFactory.createTitledBorder(I18n.get("proxy.title")));
        setLayout(new BoxLayout(this, BoxLayout.Y_AXIS));

        // 第一行：单选 + 输入 + 按钮
        JPanel row1 = new JPanel(new FlowLayout(FlowLayout.LEFT, 6, 4));

        noneRadio = new JRadioButton(I18n.get("proxy.none"));
        manualRadio = new JRadioButton(I18n.get("proxy.manual"));
        ButtonGroup group = new ButtonGroup();
        group.add(noneRadio); group.add(manualRadio);
        if (ProxyConfig.isEnabled()) manualRadio.setSelected(true);
        else noneRadio.setSelected(true);

        noneRadio.addActionListener(e -> disableProxy());
        manualRadio.addActionListener(e -> enableManual());

        row1.add(noneRadio);
        row1.add(manualRadio);

        hostField = new JTextField(ProxyConfig.getProxyHost() != null ? ProxyConfig.getProxyHost() : "127.0.0.1", 10);
        portSpinner = new JSpinner(new SpinnerNumberModel(
                ProxyConfig.getProxyPort() > 0 ? ProxyConfig.getProxyPort() : 7890, 1, 65535, 1));

        row1.add(new JLabel(I18n.get("proxy.host")));
        row1.add(hostField);
        row1.add(new JLabel(I18n.get("proxy.port")));
        row1.add(portSpinner);

        testBtn = new JButton(I18n.get("proxy.test"));
        testBtn.addActionListener(e -> testProxy());
        detectBtn = new JButton(I18n.get("proxy.autodetect"));
        detectBtn.addActionListener(e -> autoDetect());
        row1.add(testBtn);
        row1.add(detectBtn);

        add(row1);

        // 第二行：状态（系统代理提示 / 代理已禁用 共享此位置，互斥）
        JPanel statusRow = new JPanel(new FlowLayout(FlowLayout.LEFT, 0, 0));
        statusRow.setOpaque(false);
        statusLabel = new JLabel(" ");
        if (ProxyConfig.isFromSystemProxy()) {
            statusLabel.setText("已检测到Windows系统代理，自动使用生效中");
            statusLabel.setForeground(new Color(0, 128, 0));
            statusLabel.setFont(statusLabel.getFont().deriveFont(11f));
        }
        statusRow.add(statusLabel);
        add(statusRow);

        enableFields(ProxyConfig.isEnabled());
    }

    private void enableFields(boolean enabled) {
        hostField.setEnabled(enabled);
        portSpinner.setEnabled(enabled);
        testBtn.setEnabled(enabled);
        detectBtn.setEnabled(enabled);
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
        statusLabel.setForeground(UIManager.getColor("Label.foreground"));
        statusLabel.setFont(statusLabel.getFont().deriveFont(Font.PLAIN, 12f));
    }

    private void enableManual() {
        enableFields(true);
        statusLabel.setText("请输入手动代理");
        statusLabel.setForeground(UIManager.getColor("Label.foreground"));
        statusLabel.setFont(statusLabel.getFont().deriveFont(Font.PLAIN, 12f));
    }
}
