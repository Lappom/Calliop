import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useInputDevices } from "../../hooks/useInputDevices";
import { useMicProbe } from "../../hooks/useMicProbe";
import { translateError } from "../../lib/translateError";
import { Button } from "../ui/Button";
import { Select, type SelectOption } from "../ui/Select";

const DEFAULT_DEVICE_ID = "default";

interface AudioInputPanelProps {
  value: string;
  disabled?: boolean;
  onChange: (deviceId: string) => void;
}

export function AudioInputPanel({
  value,
  disabled = false,
  onChange,
}: AudioInputPanelProps) {
  const { t } = useTranslation();
  const { devices, loading, error, refresh } = useInputDevices();
  const {
    audioLevel,
    micProbing,
    micProbeStopping,
    startMicProbe,
    stopMicProbe,
  } = useMicProbe();

  useEffect(() => {
    return () => {
      void invoke("stop_mic_probe").catch(() => {});
    };
  }, []);

  const options = useMemo((): SelectOption<string>[] => {
    const entries: SelectOption<string>[] = [
      {
        value: DEFAULT_DEVICE_ID,
        label: t("settings.inputDevice.default"),
      },
    ];

    for (const device of devices) {
      entries.push({
        value: device.id,
        label: device.label,
        status: device.is_default ? "active" : undefined,
        statusLabel: device.is_default
          ? t("settings.inputDevice.defaultBadge")
          : undefined,
      });
    }

    return entries;
  }, [devices, t]);

  const selectedValue =
    options.some((option) => option.value === value) ? value : DEFAULT_DEVICE_ID;

  const showRefresh = !loading && (devices.length === 0 || error !== null);

  return (
    <div className="flex flex-col gap-3">
      <Select
        id="input-device"
        label={t("settings.inputDevice.label")}
        value={selectedValue}
        disabled={disabled || loading}
        options={options}
        onChange={onChange}
      />

      {error && (
        <p className="text-body-sm text-accent-red">
          {translateError(error, t)}
        </p>
      )}

      {!loading && devices.length === 0 && !error && (
        <p className="text-body-sm text-ash">{t("settings.inputDevice.empty")}</p>
      )}

      {showRefresh && (
        <Button
          variant="ghost"
          disabled={disabled || loading}
          onClick={() => {
            void refresh();
          }}
        >
          {t("common.refreshList")}
        </Button>
      )}

      <div
        className="h-2 overflow-hidden rounded-full bg-surface-elevated"
        aria-hidden={!micProbing}
      >
        <div
          className="h-full bg-accent-blue transition-[width] duration-75"
          style={{
            width: `${Math.min(100, Math.round(audioLevel * 100))}%`,
          }}
        />
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <Button
          variant={micProbing ? "primary" : "ghost"}
          disabled={disabled || micProbeStopping}
          onClick={() => {
            if (micProbing) {
              void stopMicProbe();
            } else {
              void startMicProbe();
            }
          }}
        >
          {micProbing
            ? t("settings.inputDevice.stopTest")
            : t("settings.inputDevice.test")}
        </Button>
        <p className="text-caption text-ash">{t("settings.inputDevice.hint")}</p>
      </div>
    </div>
  );
}
