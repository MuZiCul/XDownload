package ui.gui.panels;

import util.I18n;

import javax.swing.*;
import java.awt.*;
import java.awt.datatransfer.DataFlavor;

public class URLInputPanel extends JPanel {

    private final JTextField urlField;
    private final JButton fetchBtn, pasteBtn;
    private final DownloadPanel parent;

    public URLInputPanel(DownloadPanel parent) {
        super(new BorderLayout(5, 0));
        this.parent = parent;

        add(new JLabel(I18n.get("url.label")), BorderLayout.WEST);
        urlField = new JTextField();
        fetchBtn = new JButton(I18n.get("url.fetch"));
        pasteBtn = new JButton(I18n.get("url.paste"));

        fetchBtn.addActionListener(e -> {
            String url = urlField.getText().trim();
            if (!url.isEmpty()) parent.fetchVideoInfo(url);
        });
        urlField.addActionListener(e -> fetchBtn.doClick());

        pasteBtn.addActionListener(e -> {
            try {
                String clip = (String) Toolkit.getDefaultToolkit().getSystemClipboard()
                        .getContents(null).getTransferData(DataFlavor.stringFlavor);
                if (clip != null && !clip.isEmpty()) { urlField.setText(clip); fetchBtn.doClick(); }
            } catch (Exception ignored) {}
        });

        JPanel btnPanel = new JPanel(new GridLayout(1, 2, 4, 0));
        btnPanel.add(fetchBtn);
        btnPanel.add(pasteBtn);
        add(urlField, BorderLayout.CENTER);
        add(btnPanel, BorderLayout.EAST);
    }
}
