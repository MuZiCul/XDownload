package ui.gui.panels;

import util.I18n;
import model.VideoInfo;

import javax.swing.*;
import java.awt.*;

/** 视频元数据显示 */
public class VideoInfoPanel extends JPanel {

    private final JTextArea titleArea;
    private final JLabel uploaderLabel, durationLabel, viewsLabel;

    public VideoInfoPanel() {
        setLayout(new BoxLayout(this, BoxLayout.Y_AXIS));
        setBorder(BorderFactory.createTitledBorder(I18n.get("video.info")));

        titleArea = new JTextArea(2, 40);
        titleArea.setEditable(false);
        titleArea.setLineWrap(true);
        titleArea.setWrapStyleWord(true);
        titleArea.setBackground(getBackground());
        titleArea.setFont(titleArea.getFont().deriveFont(Font.BOLD, 14f));
        add(new JScrollPane(titleArea) {{
            setPreferredSize(new Dimension(400, 50));
            setBorder(null);
        }});

        uploaderLabel = new JLabel(" ");
        durationLabel = new JLabel(" ");
        viewsLabel = new JLabel(" ");
        add(uploaderLabel);
        add(durationLabel);
        add(viewsLabel);
    }

    public void setVideoInfo(VideoInfo info) {
        titleArea.setText(info.getTitle() != null ? info.getTitle() : I18n.get("common.unknown"));
        uploaderLabel.setText(I18n.get("video.author") + " " + (info.getUploader() != null ? info.getUploader() : I18n.get("common.unknown")));
        durationLabel.setText(I18n.get("video.duration") + " " + VideoInfo.formatDuration(info.getDuration()));
        viewsLabel.setText(I18n.get("video.views") + " " + VideoInfo.formatNumber(info.getViewCount()));
    }
}
