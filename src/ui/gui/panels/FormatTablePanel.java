package ui.gui.panels;

import util.I18n;
import model.VideoInfo;
import model.VideoInfo.Format;

import javax.swing.*;
import javax.swing.table.AbstractTableModel;
import java.awt.*;
import java.util.List;

/** 格式列表 JTable + 快捷选择按钮 */
public class FormatTablePanel extends JPanel {

    private final JTable table;
    private final FormatTableModel tableModel;
    private final DownloadPanel parent;

    public FormatTablePanel(DownloadPanel parent) {
        super(new BorderLayout());
        this.parent = parent;
        setBorder(BorderFactory.createTitledBorder(I18n.get("format.title")));

        tableModel = new FormatTableModel();
        table = new JTable(tableModel);
        table.setSelectionMode(ListSelectionModel.SINGLE_SELECTION);
        table.getSelectionModel().addListSelectionListener(e -> {
            if (!e.getValueIsAdjusting()) {
                int row = table.getSelectedRow();
                if (row >= 0) {
                    parent.onFormatSelected(tableModel.getFormatId(row));
                }
            }
        });
        JScrollPane scrollPane = new JScrollPane(table);
        scrollPane.setPreferredSize(new Dimension(400, 200));
        add(scrollPane, BorderLayout.CENTER);

        // 快捷按钮
        JPanel btnPanel = new JPanel(new FlowLayout(FlowLayout.LEFT, 4, 4));
        JButton bestBtn = new JButton(I18n.get("format.best"));
        JButton bestVABtn = new JButton(I18n.get("format.bestva"));
        JButton audioBtn = new JButton(I18n.get("format.audio"));
        bestBtn.addActionListener(e -> parent.onFormatSelected("best"));
        bestVABtn.addActionListener(e -> parent.onFormatSelected("bestvideo+bestaudio/best"));
        audioBtn.addActionListener(e -> { parent.onFormatSelected("bestaudio"); });

        btnPanel.add(bestBtn);
        btnPanel.add(bestVABtn);
        btnPanel.add(audioBtn);
        add(btnPanel, BorderLayout.SOUTH);
    }

    public void setFormats(List<Format> formats) {
        tableModel.setFormats(formats);
    }

    /** JTable 数据模型 */
    static class FormatTableModel extends AbstractTableModel {
        private String[] cols() { return new String[]{
            I18n.get("table.id"), I18n.get("table.ext"), I18n.get("table.res"),
            I18n.get("table.size"), I18n.get("table.type")}; }
        private List<Format> formats = List.of();

        public void setFormats(List<Format> formats) { this.formats = formats; fireTableDataChanged(); }
        public String getFormatId(int row) { return formats.get(row).getFormatId(); }

        @Override public int getRowCount() { return formats.size(); }
        @Override public int getColumnCount() { return cols().length; }
        @Override public String getColumnName(int col) { return cols()[col]; }

        @Override
        public Object getValueAt(int row, int col) {
            Format f = formats.get(row);
            switch (col) {
                case 0: return f.getFormatId();
                case 1: return f.getExtension() != null ? f.getExtension() : "?";
                case 2: {
                    String r = f.getResolution();
                    if (r != null) return r;
                    return f.hasVideo() ? (f.getWidth() + "x" + f.getHeight()) : "audio only";
                }
                case 3: return f.getFileSize() > 0 ? VideoInfo.formatFileSize(f.getFileSize()) : "?";
                case 4: {
                    if (f.hasVideo() && f.hasAudio()) return "V+A";
                    if (f.hasVideo()) return "V";
                    if (f.hasAudio()) return "A";
                    return "?";
                }
                default: return "";
            }
        }
    }
}
