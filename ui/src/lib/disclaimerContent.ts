/** 免责声明文案（与组件分离，避免破坏 React Fast Refresh 的 HMR）。 */

export type Lang = "zh" | "en";

export const ISSUES_URL = "https://github.com/MuZiCul/XDownload/issues";

export interface DisclaimerContent {
  title: string;
  items: string[];
  footer: string;
  complaint: { title: string; body: string[]; button: string };
  uninstall: {
    button: string;
    modalTitle: string;
    modalBody: string;
    confirm: string;
    confirming: string;
    cancel: string;
    panelHint: string;
  };
  disclaimer: {
    accept: string;
    decline: string;
    declineModalTitle: string;
    declineModalBody: string;
    confirm: string;
    confirming: string;
    cancel: string;
  };
}

export const CONTENT: Record<Lang, DisclaimerContent> = {
  zh: {
    title: "免责声明",
    items: [
      "本软件（XDownload）仅供个人学习、技术研究与合法内容存档使用，请勿将其用于任何商业用途或非法目的。",
      "请尊重版权及相关权利人的合法权益。未经权利人授权，请勿下载、复制、传播任何受版权保护的内容。用户需自行确认并确保其下载、使用的内容不侵犯任何第三方的知识产权。",
      "使用本软件时，请遵守 X/Twitter 及相关平台的服务条款，以及您所在国家/地区的法律法规。因违反上述条款或法律导致的账号封禁、内容删除或其他后果，由用户自行承担。",
      "本软件按“现状”（AS IS）提供，不提供任何明示或暗示的担保。作者及开发者不对因使用本软件而产生的任何直接、间接、偶然或必然损失（包括但不限于数据丢失、设备损坏、账号封禁、法律责任）承担任何责任。",
      "用户应对其使用本软件的全部行为及由此产生的后果负全部责任。",
      "X/Twitter、yt-dlp、ffmpeg 等名称及标识均为其各自所有者的商标，本软件与上述平台或项目无任何隶属、背书或关联关系。",
      "本软件基于 MIT 开源许可证发布，您可以依据该许可证的条款自由使用、修改与分发。",
    ],
    footer: "如您不同意上述任何条款，请立即停止使用本软件。",
    complaint: {
      title: "侵权投诉",
      body: [
        "如您认为本软件涉及对您或他人合法权益（包括但不限于版权）的侵犯，欢迎通过以下渠道与我们联系：",
        "请在项目的 GitHub Issues 中提交投诉，我们将在收到后尽快处理。",
      ],
      button: "前往提交 Issues",
    },
    uninstall: {
      button: "卸载本软件",
      modalTitle: "卸载 XDownload",
      modalBody:
        "确定要卸载 XDownload 吗？卸载将删除应用及相关文件，此操作不可恢复。",
      confirm: "确认卸载",
      confirming: "正在卸载...",
      cancel: "取消",
      panelHint: "当前为开发/便携模式，已打开系统卸载面板",
    },
    disclaimer: {
      accept: "接受 / Accept",
      decline: "不接受 / I Don't Accept",
      declineModalTitle: "退出 XDownload",
      declineModalBody: "确定要卸载并退出 XDownload 吗？此操作不可恢复。",
      confirm: "确认",
      confirming: "正在处理...",
      cancel: "取消",
    },
  },
  en: {
    title: "Disclaimer",
    items: [
      "XDownload is provided for personal learning, technical research, and lawful content archiving only.",
      "Respect copyright and intellectual property rights. Do not download, copy, or distribute copyrighted content without authorization.",
      "You are solely responsible for complying with X/Twitter's Terms of Service and all applicable laws.",
      'The software is provided "AS IS" without warranties of any kind. The author shall not be liable for any damages arising from its use.',
      "You bear full responsibility for your use of this software and its consequences.",
      "X/Twitter, yt-dlp, and ffmpeg are trademarks of their respective owners. This software has no affiliation with them.",
      "This software is released under the MIT License.",
    ],
    footer:
      "If you do not agree with any of the above terms, please stop using this software immediately.",
    complaint: {
      title: "Copyright Complaint",
      body: [
        "If you believe this software infringes your or others' legal rights (including copyright), please contact us:",
        "Submit a complaint in the project's GitHub Issues. We will handle it promptly.",
      ],
      button: "Go to Issues",
    },
    uninstall: {
      button: "Uninstall",
      modalTitle: "Uninstall XDownload",
      modalBody:
        "Are you sure you want to uninstall XDownload? Uninstalling will remove the app and related files. This action cannot be undone.",
      confirm: "Uninstall",
      confirming: "Uninstalling...",
      cancel: "Cancel",
      panelHint:
        "This is a dev/portable build. The system uninstall panel has been opened.",
    },
    disclaimer: {
      accept: "Accept",
      decline: "I Don't Accept",
      declineModalTitle: "Exit XDownload",
      declineModalBody:
        "Are you sure you want to uninstall and exit XDownload? This action cannot be undone.",
      confirm: "Confirm",
      confirming: "Processing...",
      cancel: "Cancel",
    },
  },
};
