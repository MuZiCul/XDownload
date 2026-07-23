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

        add(new ProxySettingsPanel(mainFrame));
        add(Box.createVerticalStrut(8));
        add(new CookiesSettingsPanel(mainFrame));
        add(Box.createVerticalStrut(8));
        add(buildLanguagePanel(mainFrame));
        add(Box.createVerticalGlue());
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
