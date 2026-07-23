package ui.gui.panels;

import model.VideoInfo.Format;
import util.I18n;

import javax.swing.*;
import javax.swing.table.AbstractTableModel;
import java.awt.*;
import java.util.List;

public class FormatTablePanel extends JPanel {

    private final JTable table;
    private final FormatTableModel tableModel;
    private final DownloadPanel parent;
    public final JRadioButton bestRadio, bestVARadio, audioRadio;

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
                if (row >= 0) { parent.onFormatSelected(tableModel.getFormatId(row)); }
            }
        });
        JScrollPane scrollPane = new JScrollPane(table);
        scrollPane.setPreferredSize(new Dimension(400, 200));
        add(scrollPane, BorderLayout.CENTER);

        // 单选按钮组（放在右边面板，这里只创建引用）
        ButtonGroup group = new ButtonGroup();
        bestRadio = new JRadioButton(I18n.get("format.best"));
        bestVARadio = new JRadioButton(I18n.get("format.bestva"));
        audioRadio = new JRadioButton(I18n.get("format.audio"));
        group.add(bestRadio); group.add(bestVARadio); group.add(audioRadio);
    }

    public void setFormats(List<Format> formats) {
        tableModel.setFormats(formats);
        if (!formats.isEmpty()) bestRadio.setSelected(true);
    }

    static class FormatTableModel extends AbstractTableModel {
        private static final String[] COLS = {"ID", "Ext", "Resolution"};
        private List<Format> formats = List.of();
        public void setFormats(List<Format> formats) { this.formats = formats; fireTableDataChanged(); }
        public String getFormatId(int row) { return formats.get(row).getFormatId(); }
        @Override public int getRowCount() { return formats.size(); }
        @Override public int getColumnCount() { return 3; }
        @Override public String getColumnName(int col) { return COLS[col]; }
        @Override
        public Object getValueAt(int row, int col) {
            Format f = formats.get(row);
            switch (col) {
                case 0: return f.getFormatId();
                case 1: return f.getExtension() != null ? f.getExtension() : "?";
                case 2:
                    if (f.getResolution() != null) return f.getResolution();
                    if (f.getHeight() != null) return f.getWidth() + "x" + f.getHeight();
                    return f.hasVideo() ? "video" : "audio only";
                default: return "";
            }
        }
    }
}
