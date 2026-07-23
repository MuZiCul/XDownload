package ui.gui.panels;

import util.I18n;
import util.Version;

import javax.swing.*;
import java.awt.*;
import java.net.URI;

public class AboutPanel extends JPanel {

    public AboutPanel() {
        setLayout(new BoxLayout(this, BoxLayout.Y_AXIS));
        setBorder(BorderFactory.createEmptyBorder(40, 30, 40, 30));

        JLabel title = new JLabel("XDownload");
        title.setFont(title.getFont().deriveFont(Font.BOLD, 28f));
        title.setAlignmentX(CENTER_ALIGNMENT);
        add(title);

        JLabel version = new JLabel("v" + Version.CURRENT);
        version.setFont(version.getFont().deriveFont(14f));
        version.setAlignmentX(CENTER_ALIGNMENT);
        add(version);

        add(Box.createVerticalStrut(20));

        JLabel desc = new JLabel(I18n.get("about.desc"));
        desc.setAlignmentX(CENTER_ALIGNMENT);
        add(desc);

        JLabel author = new JLabel("By MuZiCul");
        author.setAlignmentX(CENTER_ALIGNMENT);
        add(author);

        add(Box.createVerticalStrut(10));

        JLabel github = new JLabel("开源地址: github.com/MuZiCul/XDownload");
        github.setMaximumSize(new Dimension(Integer.MAX_VALUE, github.getPreferredSize().height));
        github.setAlignmentX(CENTER_ALIGNMENT);
        github.setHorizontalAlignment(SwingConstants.CENTER);
        github.setCursor(java.awt.Cursor.getPredefinedCursor(java.awt.Cursor.HAND_CURSOR));
        github.setForeground(new Color(0, 100, 200));
        github.addMouseListener(new java.awt.event.MouseAdapter() {
            public void mouseClicked(java.awt.event.MouseEvent e) {
                try { java.awt.Desktop.getDesktop().browse(java.net.URI.create("https://github.com/MuZiCul/XDownload")); }
                catch (Exception ignored) {}
            }
        });
        add(github);
    }
}
