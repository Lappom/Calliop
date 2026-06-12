import { useTranslation } from "react-i18next";
import { type AppUpdateStatus, useAppUpdate } from "../../hooks/useAppUpdate";
import { Button } from "../ui/Button";
import { ProgressBar } from "../ui/ProgressBar";
import { SettingsToggleRow } from "./SettingsToggleRow";

interface UpdatesSettingsPanelProps {
  appVersion: string | null;
  autoUpdate: boolean;
  disabled: boolean;
  onAutoUpdateChange: (checked: boolean) => void;
}

function statusDotClass(status: AppUpdateStatus): string {
  if (status === "upToDate") {
    return "bg-accent-green";
  }
  if (status === "ready" || status === "downloading") {
    return "bg-accent-blue";
  }
  if (status === "error") {
    return "bg-accent-red";
  }
  return "bg-ash";
}

export function UpdatesSettingsPanel({
  appVersion,
  autoUpdate,
  disabled,
  onAutoUpdateChange,
}: UpdatesSettingsPanelProps) {
  const { t } = useTranslation();
  const {
    status,
    pendingVersion,
    downloadProgress,
    errorMessage,
    installing,
    isBusy,
    checkForUpdates,
    installUpdate,
  } = useAppUpdate();

  const showStatus = status !== "idle";
  const statusMessage = (() => {
    switch (status) {
      case "checking":
        return t("settings.updatesPanel.checking");
      case "upToDate":
        return t("settings.updatesPanel.upToDate", {
          version: appVersion ? `v${appVersion}` : "—",
        });
      case "downloading":
        return t("settings.updatesPanel.downloading", {
          version: pendingVersion ? `v${pendingVersion}` : "—",
        });
      case "ready":
        return t("settings.updatesPanel.ready", {
          version: pendingVersion ? `v${pendingVersion}` : "—",
        });
      case "devUnavailable":
        return t("settings.updatesPanel.devUnavailable");
      case "error":
        return errorMessage ?? t("settings.updatesPanel.error");
      default:
        return null;
    }
  })();

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-body-sm m-0 font-medium text-ink">
            {t("settings.updatesPanel.currentVersion")}
          </p>
          {appVersion ? (
            <span className="mt-2 inline-flex rounded-full border border-hairline-strong bg-surface-elevated px-2.5 py-1 text-caption tabular-nums text-charcoal">
              v{appVersion}
            </span>
          ) : (
            <span className="text-caption mt-2 inline-block text-ash">—</span>
          )}
        </div>
        <Button
          variant="ghost"
          disabled={disabled || isBusy}
          onClick={() => {
            void checkForUpdates();
          }}
        >
          {status === "checking"
            ? t("settings.updatesPanel.checking")
            : t("settings.updatesPanel.check")}
        </Button>
      </div>

      <SettingsToggleRow
        label={t("settings.autoUpdate.label")}
        description={
          autoUpdate
            ? t("settings.autoUpdate.descriptionOn")
            : t("settings.autoUpdate.descriptionOff")
        }
        checked={autoUpdate}
        disabled={disabled}
        onCheckedChange={onAutoUpdateChange}
      />

      {showStatus && statusMessage ? (
        <div
          className="space-y-3 transition-[opacity,transform] duration-150 ease-out motion-reduce:transition-none"
          role="status"
        >
          <div className="flex items-start gap-2.5">
            <span
              className={[
                "mt-1.5 size-2 shrink-0 rounded-full",
                statusDotClass(status),
              ].join(" ")}
              aria-hidden
            />
            <p
              className={[
                "text-body-sm m-0",
                status === "error" ? "text-accent-red" : "text-charcoal",
              ].join(" ")}
            >
              {statusMessage}
            </p>
          </div>

          {status === "downloading" && downloadProgress !== null ? (
            <ProgressBar
              value={downloadProgress}
              label={t("settings.updatesPanel.downloading", {
                version: pendingVersion ? `v${pendingVersion}` : "—",
              })}
            />
          ) : null}

          {status === "ready" ? (
            <Button
              disabled={installing}
              onClick={() => {
                void installUpdate();
              }}
            >
              {installing
                ? t("settings.updatesPanel.installing")
                : t("settings.updatesPanel.install")}
            </Button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
