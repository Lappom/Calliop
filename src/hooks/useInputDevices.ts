import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

export interface InputDeviceInfo {
  id: string;
  label: string;
  is_default: boolean;
}

export function useInputDevices() {
  const [devices, setDevices] = useState<InputDeviceInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const listed = await invoke<InputDeviceInfo[]>("list_input_devices");
      setDevices(listed);
    } catch (err) {
      setError(String(err));
      setDevices([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { devices, loading, error, refresh };
}
