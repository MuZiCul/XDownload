import { useState } from "react";
import {
  getUninstallInfo,
  openUninstallPanel,
  uninstallApp,
} from "../../lib/bindings";
import { toast } from "sonner";
import { ExternalLink, Loader2, Trash2, X } from "lucide-react";
import { useI18n } from "../../lib/i18n";
import { CONTENT, ISSUES_URL } from "../../lib/disclaimerContent";

export default function DisclaimerPage() {
  const { lang } = useI18n();
  const [showUninstallModal, setShowUninstallModal] = useState(false);
  const [uninstalling, setUninstalling] = useState(false);

  const t = CONTENT[lang];

  const handleConfirmUninstall = async () => {
    if (uninstalling) return;
    setUninstalling(true);
    try {
      const info = await getUninstallInfo();
      if (info.installed) {
        const handled = await uninstallApp();
        // handled === true → the uninstaller was launched and the app is exiting.
        if (!handled) {
          // Installed entry exists but no usable UninstallString / launch failed →
          // fall back to the system uninstall panel.
          await openUninstallPanel();
          toast.success(t.uninstall.panelHint);
          setShowUninstallModal(false);
        }
        return;
      }
      // Not registered (dev / portable build) → open the system uninstall panel.
      await openUninstallPanel();
      toast.success(t.uninstall.panelHint);
      setShowUninstallModal(false);
    } catch (err: any) {
      toast.error(`${err}`);
    } finally {
      setUninstalling(false);
    }
  };

  return (
    <div className="h-full overflow-y-auto flex">
      <div className="min-h-full w-full flex items-center justify-center p-4">
        <div className="w-[90%] max-w-[90%]">
          <div className="section-card text-left">
            <div className="text-[13px] font-semibold text-zinc-800 mb-3">
              {t.title}
            </div>

            <ol className="list-decimal pl-5 space-y-2 text-[13px] leading-relaxed text-gray-700">
              {t.items.map((item, i) => (
                <li key={i}>{item}</li>
              ))}
            </ol>

            {/* "不同意条款" footer text + uninstall button on the same row,
                button right-aligned */}
            <div className="mt-3 flex items-center justify-between gap-3 flex-wrap">
              <p className="text-[13px] font-medium text-gray-800">
                {t.footer}
              </p>
              <button
                className="btn btn-danger flex items-center gap-1.5"
                onClick={() => setShowUninstallModal(true)}
              >
                <Trash2 size={13} />
                {t.uninstall.button}
              </button>
            </div>

            {/* Copyright complaint channel */}
            <div className="mt-6 pt-5 border-t border-zinc-200">
              <div className="text-[13px] font-semibold text-zinc-800 mb-2">
                {t.complaint.title}
              </div>
              {t.complaint.body.map((line, i) => (
                <p
                  key={i}
                  className="text-[13px] leading-relaxed text-gray-700 mb-1"
                >
                  {line}
                </p>
              ))}
              <div className="mt-3 flex justify-end">
                <a
                  href={ISSUES_URL}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 px-4 py-2 text-xs rounded-lg bg-blue-500 text-white font-medium hover:bg-blue-600 transition-colors"
                >
                  {t.complaint.button}
                  <ExternalLink size={13} />
                </a>
              </div>
            </div>
          </div>
        </div>

        {/* Uninstall confirmation modal */}
      {showUninstallModal && (
        <div
          className="dialog-overlay"
          onClick={() => !uninstalling && setShowUninstallModal(false)}
        >
          <div
            className="dialog-content"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-semibold text-zinc-900">
                {t.uninstall.modalTitle}
              </h3>
              <button
                className="text-zinc-400 hover:text-zinc-600 transition-colors"
                onClick={() => setShowUninstallModal(false)}
                disabled={uninstalling}
              >
                <X size={16} />
              </button>
            </div>
            <p className="text-xs text-zinc-500 mb-5 leading-relaxed">
              {t.uninstall.modalBody}
            </p>
            <div className="flex gap-2 justify-end">
              <button
                className="btn"
                onClick={() => setShowUninstallModal(false)}
                disabled={uninstalling}
              >
                {t.uninstall.cancel}
              </button>
              <button
                className="btn btn-danger flex items-center gap-1.5"
                onClick={handleConfirmUninstall}
                disabled={uninstalling}
              >
                {uninstalling ? (
                  <>
                    <Loader2 size={13} className="animate-spin" />
                    {t.uninstall.confirming}
                  </>
                ) : (
                  <>
                    <Trash2 size={13} />
                    {t.uninstall.confirm}
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      )}
      </div>
    </div>
  );
}
