import React, { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog, save, ask } from "@tauri-apps/plugin-dialog";
import { open } from "@tauri-apps/plugin-shell";
import { Settings, MonitorInfo } from "../App";
import { PlatformInfo } from "../config";
import { WindowExclusionTab } from "./WindowExclusionTab";
import { hexToRgba, rgbaToBgrU32, rgbaToHex } from "../utils/colors";
import { SHORTCUTS_ENABLED } from "../featureFlags";

interface CaptureRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface SaveSettingsResult {
  ok: boolean;
  message?: string;
  settings?: Settings;
}

interface SettingsDialogProps {
  initialTab?: TabType;
  settings: Settings;
  platformInfo: PlatformInfo;
  captureRegion: CaptureRegion;
  monitors: MonitorInfo[];
  selectedMonitor: number;
  onSave: (settings: Settings) => Promise<SaveSettingsResult>;
  onRegionChange: (region: CaptureRegion) => void;
  onMonitorChange: (index: number) => void;
  onClose: () => void;
}

type TabType = "capture" | "mouse" | "visual" | "shortcuts" | "region" | "performance" | "share_content" | "profiles" | "advanced" | "about";

const SectionCard = ({ title, children, className = "" }: { title: string; children: React.ReactNode; className?: string }) => (
  <div className={`bg-gray-800/50 rounded-xl p-5 border border-gray-700 shadow-sm ${className}`}>
    <h3 className="text-lg font-bold text-gray-200 mb-4">{title}</h3>
    {children}
  </div>
);

function SettingsDialog({
  initialTab = "capture",
  settings,
  platformInfo,
  captureRegion,
  monitors,
  selectedMonitor,
  onSave,
  onRegionChange,
  onMonitorChange,
  onClose
}: SettingsDialogProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabType>(initialTab);
  const [localSettings, setLocalSettings] = useState<Settings>(settings);
  const [localRegion, setLocalRegion] = useState<CaptureRegion>(captureRegion);
  const [localMonitor, setLocalMonitor] = useState<number>(selectedMonitor);
  const [previewEnabled, setPreviewEnabled] = useState(false);
  const [positionPreset, setPositionPreset] = useState<string>("center");
  const [isSyncingFromBackend, setIsSyncingFromBackend] = useState(false);
  const [setDevMode] = useState(false);
  const [appVersion, setAppVersion] = useState("Unknown");
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const [clickHighlightTest, setClickHighlightTest] = useState<{ x: number, y: number, timestamp: number } | null>(null);
  const [recordingShortcut, setRecordingShortcut] = useState<"start_capture" | "stop_capture" | "zoom_in" | "zoom_out" | null>(null);

  const pointerDownOnBackdropRef = useRef(false);

  // Profile management states
  const [profilesLoading, setProfilesLoading] = useState(false);
  const [profileVersionData, setProfileVersionData] = useState<any>(null);
  const [availableProfiles, setAvailableProfiles] = useState<any[]>([]);
  const [selectedProfileForDetails, setSelectedProfileForDetails] = useState<string>("");
  const [profileDetails, setProfileDetails] = useState<any>(null);
  const defaultShortcuts = {
    start_capture: "CmdOrCtrl+Shift+R",
    stop_capture: "CmdOrCtrl+Shift+S",
    zoom_in: "CmdOrCtrl+Shift+Equal",
    zoom_out: "CmdOrCtrl+Shift+Minus",
  };

  useEffect(() => {
    invoke<string>("get_app_version").then(setAppVersion).catch(e => console.error(e));
  }, []);

  // Load local profiles and version when Profiles tab is opened
  useEffect(() => {
    if (activeTab === "profiles") {
      // Load local profiles list
      invoke("get_capture_profiles")
        .then((profiles) => setAvailableProfiles(profiles))
        .catch((error) => console.error("Failed to load local profiles:", error));

      // Load version.json
      if (!profileVersionData) {
        invoke("get_local_profile_version")
          .then((data) => setProfileVersionData(data))
          .catch((error) => console.error("Failed to load local version.json:", error));
      }
    }
  }, [activeTab]);

  useEffect(() => {
    if (!recordingShortcut) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        setRecordingShortcut(null);
        return;
      }

      const parts: string[] = [];
      if (event.metaKey || event.ctrlKey) {
        parts.push("CmdOrCtrl");
      }
      if (event.altKey) {
        parts.push("Alt");
      }
      if (event.shiftKey) {
        parts.push("Shift");
      }

      const isModifierOnly = ["Shift", "Control", "Alt", "Meta"].includes(event.key);
      if (isModifierOnly) {
        return;
      }

      if (parts.length === 0) {
        setToastMessage("Please include Ctrl/Cmd, Alt, or Shift.");
        return;
      }

      const key = (() => {
        if (event.code === "NumpadAdd") {
          return "NumpadAdd";
        }
        if (event.key === "+" || event.key === "=") {
          return "Equal";
        }
        if (event.code === "NumpadSubtract" || event.key === "-" || event.key === "_") {
          return "Minus";
        }
        if (event.key === " ") {
          return "Space";
        }
        if (event.key.startsWith("Arrow")) {
          return event.key.replace("Arrow", "");
        }
        if (event.key.length === 1) {
          return event.key.toUpperCase();
        }
        if (/^F\d{1,2}$/i.test(event.key)) {
          return event.key.toUpperCase();
        }
        if (["Home", "End", "PageUp", "PageDown", "Insert", "Delete"].includes(event.key)) {
          return event.key;
        }
        return null;
      })();

      if (!key) {
        setToastMessage("Unsupported key. Try letters, numbers, arrows, or function keys.");
        return;
      }

      const accelerator = [...parts, key].join("+");
      setLocalSettings((prev) => ({
        ...prev,
        shortcuts: {
          ...(prev.shortcuts || defaultShortcuts),
          [recordingShortcut]: accelerator,
        },
      }));
      setRecordingShortcut(null);
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [recordingShortcut]);

  // Auto-hide toast
  useEffect(() => {
    if (toastMessage) {
      const timer = setTimeout(() => setToastMessage(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [toastMessage]);







  // Auto-detect monitor when region changes
  useEffect(() => {
    if (monitors.length === 0) return;

    // Find which monitor contains the center of the region
    const centerX = localRegion.x + localRegion.width / 2;
    const centerY = localRegion.y + localRegion.height / 2;

    const newMonitorIndex = monitors.findIndex((mon) => {
      return (
        centerX >= mon.x &&
        centerX < mon.x + mon.width &&
        centerY >= mon.y &&
        centerY < mon.y + mon.height
      );
    });

    if (newMonitorIndex !== -1 && newMonitorIndex !== localMonitor) {
      setLocalMonitor(newMonitorIndex);
    }
  }, [localRegion, monitors]);

  // Handle ESC key to close dialog
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  // Load dev mode
  useEffect(() => {
    invoke<boolean>("is_dev_mode").then(setDevMode).catch(() => setDevMode(false));
  }, []);

  // Performance calculation
  const roundToStandardRefreshRate = (rate: number): number => {
    const standardRates = [24, 25, 30, 50, 60, 75, 90, 120, 144, 165, 240, 360];
    for (const standard of standardRates) {
      if (Math.abs(rate - standard) <= standard * 0.1) {
        return standard;
      }
    }
    return Math.round(rate);
  };

  const monitorRefreshRate = monitors[localMonitor]
    ? roundToStandardRefreshRate(monitors[localMonitor].refresh_rate)
    : 60;
  const maxFps = monitorRefreshRate * 2;

  // Prevent background scroll
  useEffect(() => {
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "unset";
    };
  }, []);

  // Preview border management - only create/destroy on toggle
  useEffect(() => {
    if (previewEnabled) {
      // Backend expects 0x00BBGGRR from [R, G, B, A]
      const borderColor = rgbaToBgrU32(localSettings.border_color);

      // Create border once when preview is enabled
      invoke("show_preview_border", {
        x: localRegion.x,
        y: localRegion.y,
        width: localRegion.width,
        height: localRegion.height,
        borderWidth: localSettings.border_width,
        borderColor: borderColor
      }).catch(console.error);
    } else {
      invoke("hide_preview_border").catch(console.error);
    }

    return () => {
      invoke("hide_preview_border").catch(console.error);
    };
  }, [previewEnabled]);

  const lastSentRegion = useRef({ x: 0, y: 0, width: 0, height: 0 });

  useEffect(() => {
    if (previewEnabled && !isSyncingFromBackend) {
      if (localRegion.x !== lastSentRegion.current.x ||
        localRegion.y !== lastSentRegion.current.y ||
        localRegion.width !== lastSentRegion.current.width ||
        localRegion.height !== lastSentRegion.current.height) {

        lastSentRegion.current = { ...localRegion };
        invoke("update_preview_border", {
          x: localRegion.x,
          y: localRegion.y,
          width: localRegion.width,
          height: localRegion.height
        }).catch(console.error);
      }
    }
  }, [localRegion, previewEnabled, isSyncingFromBackend]);

  useEffect(() => {
    if (previewEnabled) {
      // Backend expects 0x00BBGGRR from [R, G, B, A]
      const borderColor = rgbaToBgrU32(localSettings.border_color);

      invoke("update_preview_border_style", {
        borderWidth: localSettings.border_width,
        borderColor: borderColor
      }).catch(console.error);
    }
  }, [localSettings.border_color, localSettings.border_width, previewEnabled]);

  // Sync back from border movement
  useEffect(() => {
    if (!previewEnabled) return;

    let lastKnownRect = { x: localRegion.x, y: localRegion.y, width: localRegion.width, height: localRegion.height };

    const syncInterval = setInterval(async () => {
      try {
        const rect = await invoke<[number, number, number, number] | null>("get_preview_border_rect");
        if (rect) {
          const [x, y, width, height] = rect;

          if (x !== lastKnownRect.x || y !== lastKnownRect.y ||
            width !== lastKnownRect.width || height !== lastKnownRect.height) {

            lastKnownRect = { x, y, width, height };
            lastSentRegion.current = { x, y, width, height };

            setIsSyncingFromBackend(true);
            setLocalRegion({ x, y, width, height });
            setPositionPreset("custom");

            setTimeout(() => setIsSyncingFromBackend(false), 100);

            for (let i = 0; i < monitors.length; i++) {
              const mon = monitors[i];
              const centerX = x + width / 2;
              const centerY = y + height / 2;

              if (centerX >= mon.x && centerX < mon.x + mon.width &&
                centerY >= mon.y && centerY < mon.y + mon.height) {
                setLocalMonitor(i);
                break;
              }
            }
          }
        }
      } catch (e) { }
    }, 300);

    return () => clearInterval(syncInterval);
  }, [previewEnabled, monitors]);

  const handleSave = async () => {
    const result = await onSave(localSettings);
    if (!result.ok) {
      if (result.message) {
        setToastMessage(result.message);
      }
      if (result.settings) {
        setLocalSettings(result.settings);
      }
      return;
    }

    onRegionChange(localRegion);
    onMonitorChange(localMonitor);
    onClose();
  };

  const handleExportSettings = async () => {
    try {
      const filePath = await save({
        defaultPath: "rustframe-settings.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (filePath) {
        await invoke("export_settings", { path: filePath });
        setToastMessage("Settings exported successfully");
      }
    } catch (error) {
      console.error(error);
      setToastMessage("Failed to export settings");
    }
  };

  const handleImportSettings = async () => {
    try {
      const filePath = await openDialog({
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });
      if (filePath) {
        const imported = await invoke<Settings>("import_settings", { path: filePath });
        const result = await onSave(imported);
        if (!result.ok) {
          if (result.message) {
            setToastMessage(result.message);
          } else {
            setToastMessage("Failed to import settings");
          }
          if (result.settings) {
            setLocalSettings(result.settings);
          }
          return;
        }

        setLocalSettings(imported);
        setToastMessage("Settings imported successfully");
      }
    } catch (error) {
      console.error(error);
      setToastMessage("Failed to import settings");
    }
  };

  const tabs: { id: TabType; label: string; icon: string }[] = [
    { id: "capture" as TabType, label: t('settings.tabs.capture'), icon: "🎯" },
    { id: "mouse" as TabType, label: t('settings.tabs.mouse'), icon: "🖱️" },
    { id: "visual" as TabType, label: t('settings.tabs.visual'), icon: "🎨" },
    { id: "shortcuts" as TabType, label: t('settings.tabs.shortcuts'), icon: "⌨️" },
    { id: "region" as TabType, label: t('settings.tabs.region'), icon: "📐" },
    { id: "performance" as TabType, label: t('settings.tabs.performance'), icon: "🚀" },
    { id: "share_content" as TabType, label: t('settings.tabs.share_content'), icon: "📺" },
    { id: "profiles" as TabType, label: t('settings.tabs.profiles'), icon: "📦" },
    { id: "advanced" as TabType, label: t('settings.tabs.advanced'), icon: "🔧" },
    { id: "about" as TabType, label: t('settings.tabs.about'), icon: "ℹ️" },
  ].filter((tab) => SHORTCUTS_ENABLED || tab.id !== "shortcuts");

  return (
    <div
      className="fixed inset-0 bg-black/80 flex items-center justify-center z-50"
      style={{ WebkitAppRegion: 'no-drag' } as any} onMouseDown={(e) => e.stopPropagation()}
      onPointerDownCapture={(e) => {
        pointerDownOnBackdropRef.current = e.target === e.currentTarget;
      }}
      onClick={(e) => {
        // Close only if the *press* started on the backdrop.
        // This prevents a slider drag that ends outside the input from closing the modal.
        if (e.target === e.currentTarget && pointerDownOnBackdropRef.current) onClose();
      }}>
      {/* Toast */}
      {toastMessage && (
        <div className="fixed top-6 right-6 z-[70] bg-gray-900 border border-gray-700 rounded-lg px-4 py-3 shadow-2xl animate-slide-in flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-green-500"></div>
          <p className="text-white text-sm font-medium">{toastMessage}</p>
        </div>
      )}

      <div
        className="bg-gray-900 rounded-2xl shadow-2xl w-full max-w-4xl max-h-[85vh] flex flex-col border border-gray-700"
        style={{ WebkitAppRegion: 'no-drag' } as any}
        onMouseDown={(e) => e.stopPropagation()}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-5 border-b border-gray-800 bg-gradient-to-r from-gray-900 to-gray-800 flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-400 to-purple-400">{t('settings.title')}</h2>
            <p className="text-gray-400 text-sm mt-1">{t('settings.subtitle')}</p>
          </div>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white transition-colors p-2 hover:bg-gray-800 rounded-lg"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Tabs Bar */}
        <div className="px-2 pt-2 border-b border-gray-800 bg-gray-900">
          <div className="flex flex-wrap gap-1 p-2">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`flex items-center gap-2 px-4 py-3 rounded-xl transition-all duration-200 font-medium text-sm flex-shrink-0 ${activeTab === tab.id
                  ? "bg-gray-800 text-white shadow-lg border border-gray-700 transform scale-[1.02]"
                  : "text-gray-400 hover:text-white hover:bg-gray-800/50 border border-transparent"
                  }`}
              >
                <span className="text-lg">{tab.icon}</span>
                {tab.label}
              </button>
            ))}
          </div>
        </div>

        {/* Content Area */}
        <div className="flex-1 p-6 bg-gray-900/50" style={{ overflow: 'auto' }}>

          {/* TAB: CAPTURE */}
          {activeTab === "capture" && (
            <div className="space-y-6 animate-fadeIn">
              {/* Platform Info */}
              <div className="bg-blue-900/10 border border-blue-500/20 rounded-xl p-4 flex items-center gap-4">
                <div className="p-3 bg-blue-500/20 rounded-lg">
                  <svg className="w-6 h-6 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                  </svg>
                </div>
                <div>
                  <div className="font-bold text-gray-200">{platformInfo.os_name} {platformInfo.os_version}</div>
                  <div className="text-sm text-gray-400">
                    {platformInfo.capabilities.supports_hardware_acceleration ? t('settings.capture.hardware_accel_active') : t('settings.capture.hardware_accel_inactive')}
                  </div>
                </div>
              </div>

              <SectionCard title={t('settings.sections.capture_method')}>
                <div className={`grid grid-cols-1 ${platformInfo.available_capture_methods.length > 1 ? 'md:grid-cols-2' : ''} gap-4`}>
                  {platformInfo.available_capture_methods.map((method) => (
                    <label
                      key={method.id}
                      className={`relative flex flex-col p-4 rounded-xl border-2 transition-all cursor-pointer hover:bg-gray-700/50 ${localSettings.capture_method === method.id
                        ? 'bg-blue-600/10 border-blue-500'
                        : 'bg-gray-900 border-gray-700'
                        }`}
                    >
                      <input
                        type="radio"
                        name="capture_method"
                        checked={localSettings.capture_method === method.id}
                        onChange={() => setLocalSettings({ ...localSettings, capture_method: method.id as any })}
                        className="absolute top-4 right-4 w-5 h-5 text-blue-600 bg-gray-700 border-gray-600 focus:ring-blue-500 focus:ring-offset-gray-900"
                      />
                      <span className="font-bold text-white mb-1">{method.id === "Wgc" ? t('settings.capture.wgc_name') : method.id === "GdiCopy" ? t('settings.capture.gdi_name') : method.name}</span>
                      <p className="text-xs text-gray-400 mb-3">{method.id === "Wgc" ? t('settings.capture.wgc_desc') : method.id === "GdiCopy" ? t('settings.capture.gdi_desc') : method.description}</p>

                      <div className="flex flex-wrap gap-2 mt-auto">
                        {method.recommended && (
                          <span className="px-2 py-1 bg-green-500/10 text-green-400 text-xs rounded-lg font-medium border border-green-500/20">
                            {t('settings.capture.recommended')}
                          </span>
                        )}
                        {method.hardware_accelerated && (
                          <span className="px-2 py-1 bg-purple-500/10 text-purple-400 text-xs rounded-lg font-medium border border-purple-500/20">
                            {t('settings.capture.gpu_accelerated')}
                          </span>
                        )}
                      </div>
                    </label>
                  ))}
                </div>
              </SectionCard>

              <SectionCard title={t('settings.sections.preview_mode')}>
                <div className="space-y-4">
                  <p className="text-sm text-gray-400 mb-2">{t('settings.capture.preview_desc')}</p>

                  {platformInfo.os_type === "windows" && (
                    <label className={`flex items-center p-4 rounded-xl border transition-all cursor-pointer ${localSettings.preview_mode === "WinApiGdi" ? 'bg-blue-600/10 border-blue-500' : 'bg-gray-900 border-gray-700 hover:bg-gray-700/50'
                      }`}>
                      <input
                        type="radio"
                        name="preview_mode"
                        checked={localSettings.preview_mode === "WinApiGdi"}
                        onChange={() => setLocalSettings({ ...localSettings, preview_mode: "WinApiGdi" })}
                        className="w-5 h-5 text-blue-600 mr-4"
                      />
                      <div>
                        <div className="font-bold text-gray-200">WinAPI GDI</div>
                        <div className="text-xs text-gray-500">{t('settings.capture.winapi_desc')}</div>
                      </div>
                    </label>
                  )}

                  <label className={`flex items-center p-4 rounded-xl border transition-all cursor-pointer ${localSettings.preview_mode === "TauriCanvas" ? 'bg-blue-600/10 border-blue-500' : 'bg-gray-900 border-gray-700 hover:bg-gray-700/50'
                    }`}>
                    <input
                      type="radio"
                      name="preview_mode"
                      checked={localSettings.preview_mode === "TauriCanvas"}
                      onChange={() => setLocalSettings({ ...localSettings, preview_mode: "TauriCanvas" })}
                      className="w-5 h-5 text-blue-600 mr-4"
                    />
                    <div>
                      <div className="font-bold text-gray-200">Tauri Canvas</div>
                      <div className="text-xs text-gray-500">{t('settings.capture.tauri_desc')}</div>
                    </div>
                  </label>
                </div>
              </SectionCard>
            </div>
          )}

          {/* TAB: MOUSE */}
          {activeTab === "mouse" && (
            <div className="space-y-6 animate-fadeIn">
              <SectionCard title={t('settings.sections.shadow_cursor')}>
                <div className="flex flex-col gap-4">
                  <label className="flex items-center justify-between p-4 bg-gray-900 rounded-lg border border-gray-700 cursor-pointer hover:bg-gray-800 transition-colors">
                    <div className="flex items-center gap-3">
                      <div className="p-2 bg-blue-500/20 rounded text-blue-400">
                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 15l-2 5L9 9l11 4-5 2zm0 0l5 5M7.188 2.239l.777 2.897M5.136 7.965l-2.898-.777M13.95 4.05l-2.122 2.122m-5.657 5.656l-2.12 2.122" />
                        </svg>
                      </div>
                      <div>
                        <span className="font-medium text-gray-200 block">{t('settings.mouse.show_cursor')}</span>
                        <span className="text-xs text-gray-500" title={t('settings.mouse.show_cursor_desc')}>{t('settings.mouse.show_cursor_desc')}</span>
                      </div>
                    </div>
                    <div className={`w-12 h-6 rounded-full p-1 transition-colors ${localSettings.show_cursor ? 'bg-blue-600' : 'bg-gray-600'}`}>
                      <input
                        type="checkbox"
                        className="hidden"
                        checked={localSettings.show_cursor}
                        onChange={(e) => setLocalSettings({ ...localSettings, show_cursor: e.target.checked })}
                      />
                      <div className={`w-4 h-4 rounded-full bg-white transition-transform ${localSettings.show_cursor ? 'translate-x-6' : ''}`}></div>
                    </div>
                  </label>

                  <label className="flex items-center justify-between p-4 bg-gray-900 rounded-lg border border-gray-700 cursor-pointer hover:bg-gray-800 transition-colors">
                    <div className="flex items-center gap-3">
                      <div className="p-2 bg-purple-500/20 rounded text-purple-400">
                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                        </svg>
                      </div>
                      <div>
                        <span className="font-medium text-gray-200 block">{t('settings.mouse.highlight_clicks')}</span>
                        <span className="text-xs text-gray-500">{t('settings.mouse.highlight_clicks_desc')}</span>
                      </div>
                    </div>
                    <div className={`w-12 h-6 rounded-full p-1 transition-colors ${localSettings.capture_clicks ? 'bg-purple-600' : 'bg-gray-600'}`}>
                      <input
                        type="checkbox"
                        className="hidden"
                        checked={localSettings.capture_clicks}
                        onChange={(e) => setLocalSettings({ ...localSettings, capture_clicks: e.target.checked })}
                      />
                      <div className={`w-4 h-4 rounded-full bg-white transition-transform ${localSettings.capture_clicks ? 'translate-x-6' : ''}`}></div>
                    </div>
                  </label>
                </div>
              </SectionCard>

              {/* Click Customization - Only if enabled */}
              <div className={`transition-all duration-300 ${localSettings.capture_clicks ? 'opacity-100 max-h-[600px]' : 'opacity-40 max-h-0 overflow-hidden pointer-events-none select-none grayscale'}`}
                onMouseDown={(e) => {
                  // Ensure mouse down in this container also stops propagation if enabled.
                  if (localSettings.capture_clicks) e.stopPropagation();
                }}
              >
                <SectionCard title={t('settings.sections.click_highlight_style')}>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                      <label className="block text-sm text-gray-400 mb-2">{t('settings.mouse.color')}</label>
                      <div className="flex items-center gap-3">
                        <input
                          type="color"
                          value={rgbaToHex(localSettings.click_highlight_color)}
                          onInput={(e) => setLocalSettings({ ...localSettings, click_highlight_color: hexToRgba((e.target as HTMLInputElement).value) })}
                          onChange={(e) => setLocalSettings({ ...localSettings, click_highlight_color: hexToRgba(e.target.value) })}
                          className="w-12 h-8 rounded cursor-pointer"
                          style={{ WebkitAppRegion: 'no-drag' } as any} onMouseDown={(e) => e.stopPropagation()}
                        />
                        <div className="text-xs text-gray-500 font-mono">
                          {rgbaToHex(localSettings.click_highlight_color)}
                        </div>
                      </div>
                    </div>

                    <div className="space-y-4">
                      <div>
                        <div className="flex justify-between text-sm mb-1">
                          <span className="text-gray-400">{t('settings.mouse.radius')}</span>
                          <span className="text-gray-200">{localSettings.click_highlight_radius}px</span>
                        </div>
                        <input
                          type="range"
                          min="10"
                          max="100"
                          value={localSettings.click_highlight_radius}
                          onChange={(e) => setLocalSettings({ ...localSettings, click_highlight_radius: parseInt(e.target.value) })}
                          className="w-full h-2 bg-gray-700 rounded-lg cursor-pointer"
                          style={{ WebkitAppRegion: 'no-drag' } as any}
                          onMouseDown={(e) => e.stopPropagation()}
                        />
                      </div>
                      <div>
                        <div className="flex justify-between text-sm mb-1">
                          <span className="text-gray-400">{t('settings.mouse.fade_duration')}</span>
                          <span className="text-gray-200">{localSettings.click_dissolve_ms}ms</span>
                        </div>
                        <input
                          type="range"
                          min="100"
                          max="2000"
                          step="100"
                          value={localSettings.click_dissolve_ms}
                          onChange={(e) => setLocalSettings({ ...localSettings, click_dissolve_ms: parseInt(e.target.value) })}
                          className="w-full h-2 bg-gray-700 rounded-lg cursor-pointer"
                          style={{ WebkitAppRegion: 'no-drag' } as any}
                          onMouseDown={(e) => e.stopPropagation()}
                        />
                      </div>
                    </div>
                  </div>

                  {/* Test Button with Preview */}
                  <div className="mt-6 border-t border-gray-700 pt-4">
                    <div className="flex justify-between items-center mb-3">
                      <label className="text-sm text-gray-400">{t('settings.mouse.test_highlight')}</label>
                      <button
                        onClick={() => {
                          // Calculate dimensions dynamically
                          const previewWidth = Math.max(150, localSettings.click_highlight_radius * 3);
                          const previewHeight = Math.max(60, localSettings.click_highlight_radius * 2.5);
                          setClickHighlightTest({
                            x: previewWidth / 2,
                            y: previewHeight / 2,
                            timestamp: Date.now()
                          });
                          setTimeout(() => setClickHighlightTest(null), localSettings.click_dissolve_ms);
                        }}
                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors text-sm font-medium flex items-center gap-2"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                        </svg>
                        {t('settings.mouse.test_highlight')}
                      </button>
                    </div>

                    {/* Preview Area (non-interactive, display only) - sized based on radius */}
                    <div
                      className="relative bg-gray-900/50 rounded-lg border border-gray-700 overflow-hidden mx-auto"
                      style={{
                        pointerEvents: 'none',
                        width: `${Math.max(150, localSettings.click_highlight_radius * 3)}px`,
                        height: `${Math.max(60, localSettings.click_highlight_radius * 2.5)}px`
                      }}
                    >
                      <div className="absolute inset-0 flex items-center justify-center text-gray-600 text-xs">
                        {t('settings.mouse.preview_test_desc')}
                      </div>

                      {/* Click Highlight Animation */}
                      {clickHighlightTest && (
                        <div
                          style={{
                            position: 'absolute',
                            left: clickHighlightTest.x,
                            top: clickHighlightTest.y,
                            width: localSettings.click_highlight_radius * 2,
                            height: localSettings.click_highlight_radius * 2,
                            marginLeft: -localSettings.click_highlight_radius,
                            marginTop: -localSettings.click_highlight_radius,
                            borderRadius: '50%',
                            backgroundColor: rgbaToHex(localSettings.click_highlight_color),
                            opacity: localSettings.click_highlight_color[3] / 255,
                            animation: `clickHighlightFade ${localSettings.click_dissolve_ms}ms ease-out forwards`,
                            pointerEvents: 'none',
                          }}
                        />
                      )}
                    </div>
                  </div>
                </SectionCard>
              </div>
            </div>
          )}

          {/* TAB: VISUAL */}
          {activeTab === "visual" && (
            <div className="space-y-6 animate-fadeIn">
              <SectionCard title={t('settings.sections.interface_scale')}>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="font-medium text-gray-200 block">{t('settings.visual.ui_zoom')}</span>
                      <span className="text-xs text-gray-500">{t('settings.visual.ui_zoom_desc')}</span>
                    </div>
                    <button
                      onClick={() => setLocalSettings({ ...localSettings, ui_zoom: 1.0 })}
                      className="px-3 py-1 bg-gray-700 hover:bg-gray-600 rounded-lg text-xs text-white transition-colors"
                    >
                      {t('settings.visual.reset_zoom')}
                    </button>
                  </div>
                  <div>
                    <input
                      type="range"
                      min="80"
                      max="125"
                      step="5"
                      value={Math.round((localSettings.ui_zoom || 1) * 100)}
                      onInput={(e) =>
                        setLocalSettings({
                          ...localSettings,
                          ui_zoom: parseInt((e.target as HTMLInputElement).value, 10) / 100,
                        })
                      }
                      onChange={(e) =>
                        setLocalSettings({
                          ...localSettings,
                          ui_zoom: parseInt(e.target.value, 10) / 100,
                        })
                      }
                      className="w-full h-2 bg-gray-700 rounded-lg cursor-pointer accent-blue-500 hover:accent-blue-400"
                      style={{ WebkitAppRegion: 'no-drag' } as any} onMouseDown={(e) => e.stopPropagation()}
                    />
                    <div className="flex justify-between text-xs text-gray-500 mt-2 font-mono">
                      <span>80%</span>
                      <span>{Math.round((localSettings.ui_zoom || 1) * 100)}%</span>
                      <span>125%</span>
                    </div>
                  </div>
                  <div className="text-xs text-gray-500">
                    {t('settings.visual.zoom_hint')}
                  </div>
                </div>
              </SectionCard>

              <SectionCard title={t('settings.sections.capture_border')}>
                <div className="space-y-6">
                  <label className="flex items-center justify-between">
                    <div>
                      <span className="font-medium text-gray-200 block">{t('settings.visual.show_border')}</span>
                      <span className="text-xs text-gray-500">{t('settings.visual.show_border_desc')}</span>
                    </div>
                    <div className={`w-12 h-6 rounded-full p-1 transition-colors ${localSettings.show_border ? 'bg-blue-600' : 'bg-gray-600'}`}>
                      <input
                        type="checkbox"
                        className="hidden"
                        checked={localSettings.show_border}
                        onChange={(e) => setLocalSettings({ ...localSettings, show_border: e.target.checked })}
                      />
                      <div className={`w-4 h-4 rounded-full bg-white transition-transform ${localSettings.show_border ? 'translate-x-6' : ''}`}></div>
                    </div>
                  </label>

                  <div className={`grid grid-cols-2 gap-6 transition-opacity ${localSettings.show_border ? 'opacity-100' : 'opacity-40 pointer-events-none'}`}>
                    <div>
                      <label className="block text-sm text-gray-400 mb-2">{t('settings.visual.border_color')}</label>
                      <input
                        type="color"
                        value={rgbaToHex(localSettings.border_color)}
                        onInput={(e) => setLocalSettings({ ...localSettings, border_color: hexToRgba((e.target as HTMLInputElement).value) })}
                        onChange={(e) => setLocalSettings({ ...localSettings, border_color: hexToRgba(e.target.value) })}
                        className="w-full h-10 rounded-lg cursor-pointer bg-gray-700"
                        style={{ WebkitAppRegion: 'no-drag' } as any} onMouseDown={(e) => e.stopPropagation()}


                      />
                    </div>
                    <div>
                      <label className="block text-sm text-gray-400 mb-2">{t('settings.visual.border_width')}: {localSettings.border_width}px</label>
                      <input
                        type="range"
                        min="1"
                        max="20"
                        value={localSettings.border_width}
                        onInput={(e) => setLocalSettings({ ...localSettings, border_width: parseInt((e.target as HTMLInputElement).value) })}
                        onChange={(e) => setLocalSettings({ ...localSettings, border_width: parseInt(e.target.value) })}
                        className="w-full h-2 bg-gray-700 rounded-lg cursor-pointer mt-2"
                        style={{ WebkitAppRegion: 'no-drag' } as any} onMouseDown={(e) => e.stopPropagation()}


                      />
                    </div>
                  </div>
                </div>
              </SectionCard>

              <SectionCard title={t('settings.sections.rec_indicator')}>
                <div className="space-y-6">
                  <label className="flex items-center justify-between">
                    <div>
                      <span className="font-medium text-gray-200 block">{t('settings.visual.show_rec')}</span>
                      <span className="text-xs text-gray-500">{t('settings.visual.show_rec_desc')}</span>
                    </div>
                    <div className={`w-12 h-6 rounded-full p-1 transition-colors ${localSettings.show_rec_indicator ? 'bg-red-600' : 'bg-gray-600'}`}>
                      <input
                        type="checkbox"
                        className="hidden"
                        checked={localSettings.show_rec_indicator}
                        onChange={(e) => setLocalSettings({ ...localSettings, show_rec_indicator: e.target.checked })}
                      />
                      <div className={`w-4 h-4 rounded-full bg-white transition-transform ${localSettings.show_rec_indicator ? 'translate-x-6' : ''}`}></div>
                    </div>
                  </label>

                  <div className={`transition-opacity ${localSettings.show_rec_indicator ? 'opacity-100' : 'opacity-40 pointer-events-none'}`}>
                    <label className="block text-sm text-gray-400 mb-2">{t('settings.visual.indicator_size')}</label>
                    <div className="flex gap-2">
                      {['small', 'medium', 'large'].map((size) => (
                        <button
                          key={size}
                          onClick={() => setLocalSettings({ ...localSettings, rec_indicator_size: size as any })}
                          className={`flex-1 py-2 rounded-lg border text-sm capitalize transition-colors ${localSettings.rec_indicator_size === size
                            ? 'bg-red-500/20 border-red-500 text-red-300'
                            : 'bg-gray-900 border-gray-700 text-gray-400 hover:bg-gray-800'
                            }`}
                        >
                          {t(`settings.visual.${size}`)}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>
              </SectionCard>
            </div>
          )}

          {/* TAB: SHORTCUTS */}
          {SHORTCUTS_ENABLED && activeTab === "shortcuts" && (
            <div className="space-y-6 animate-fadeIn">
              <SectionCard title={t('settings.sections.shortcuts')}>
                <div className="space-y-3">
                  {[
                    { key: "start_capture", label: t('settings.shortcuts.start_capture'), help: t('settings.shortcuts.desc') },
                    { key: "stop_capture", label: t('settings.shortcuts.stop_capture'), help: t('settings.shortcuts.desc') },
                    { key: "zoom_in", label: t('settings.shortcuts.zoom_in'), help: t('settings.shortcuts.desc') },
                    { key: "zoom_out", label: t('settings.shortcuts.zoom_out'), help: t('settings.shortcuts.desc') },
                  ].map((item) => {
                    const isRecording = recordingShortcut === item.key;
                    const value = localSettings.shortcuts?.[item.key as keyof typeof localSettings.shortcuts] || "";
                    return (
                      <div
                        key={item.key}
                        className="flex flex-col gap-2 rounded-xl border border-gray-700 bg-gray-800/40 p-4"
                      >
                        <div className="flex items-start justify-between gap-4">
                          <div>
                            <div className="font-medium text-gray-200">{item.label}</div>
                            <div className="text-xs text-gray-500">{item.help}</div>
                          </div>
                          <div className="flex items-center gap-2">
                            <input
                              type="text"
                              readOnly
                              value={isRecording ? t('settings.shortcuts.recording_hint') : value || "Unassigned"}
                              className={`w-48 rounded-lg border px-3 py-2 text-sm ${isRecording
                                ? "border-blue-500 bg-blue-500/10 text-blue-200"
                                : "border-gray-700 bg-gray-900 text-gray-200"
                                }`}
                            />
                            {isRecording ? (
                              <button
                                onClick={() => setRecordingShortcut(null)}
                                className="px-3 py-2 rounded-lg bg-gray-700 hover:bg-gray-600 text-white text-sm"
                              >
                                Cancel
                              </button>
                            ) : (
                              <button
                                onClick={() => setRecordingShortcut(item.key as any)}
                                className="px-3 py-2 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-sm"
                              >
                                {t('settings.shortcuts.click_hint')}
                              </button>
                            )}
                            <button
                              onClick={() =>
                                setLocalSettings((prev) => ({
                                  ...prev,
                                  shortcuts: {
                                    ...(prev.shortcuts || defaultShortcuts),
                                    [item.key]: "",
                                  },
                                }))
                              }
                              className="px-3 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm border border-gray-700"
                            >
                              {t('app.close')}
                            </button>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>

                <div className="flex items-center justify-between mt-4 text-xs text-gray-500">
                  <span>Tip: Press Esc to cancel recording.</span>
                  <button
                    onClick={() =>
                      setLocalSettings((prev) => ({
                        ...prev,
                        shortcuts: { ...defaultShortcuts },
                      }))
                    }
                    className="px-3 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm border border-gray-700"
                  >
                    {t('settings.shortcuts.reset_all')}
                  </button>
                </div>
              </SectionCard>
            </div>
          )}

          {/* TAB: REGION */}
          {activeTab === "region" && (
            <div className="space-y-6 animate-fadeIn">
              <div className="bg-yellow-500/10 border border-yellow-500/20 rounded-xl p-4 flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <svg className="w-6 h-6 text-yellow-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                  </svg>
                  <div>
                    <div className="font-bold text-gray-200">{t('settings.region.preview_border')}</div>
                    <div className="text-xs text-gray-400">{t('settings.region.preview_border_desc')}</div>
                  </div>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" className="sr-only peer" checked={previewEnabled} onChange={(e) => setPreviewEnabled(e.target.checked)} />
                  <div className="w-11 h-6 bg-gray-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-yellow-500"></div>
                </label>
              </div>

              <SectionCard title={t('settings.region.monitor')}>
                <select
                  value={localMonitor}
                  onChange={(e) => {
                    const idx = parseInt(e.target.value);
                    setLocalMonitor(idx);
                    if (monitors[idx]) {
                      const mon = monitors[idx];
                      setLocalRegion({
                        x: mon.x + Math.floor((mon.width - localRegion.width) / 2),
                        y: mon.y + Math.floor((mon.height - localRegion.height) / 2),
                        width: localRegion.width,
                        height: localRegion.height
                      });
                    }
                  }}
                  className="w-full px-4 py-3 bg-gray-900 border border-gray-700 rounded-lg focus:ring-2 focus:ring-blue-500 focus:outline-none text-white text-lg"
                >
                  {monitors.map((mon, idx) => (
                    <option key={mon.id} value={idx}>
                      {mon.name} ({mon.width}x{mon.height}{mon.scale_factor ? ` @ ${(mon.scale_factor * 100).toFixed(0)}%` : ""}) {mon.is_primary ? `⭐ ${t('settings.region.primary')}` : ""}
                    </option>
                  ))}
                </select>
              </SectionCard>

              <SectionCard title={t('settings.region.dimensions')}>
                <div className="grid grid-cols-2 gap-4 mb-4">
                  <div>
                    <label className="text-xs text-gray-500 uppercase font-bold tracking-wider mb-1 block">{t('settings.region.width')}</label>
                    <input
                      type="number"
                      value={localRegion.width}
                      onChange={(e) => setLocalRegion({ ...localRegion, width: parseInt(e.target.value) || 800 })}
                      className="w-full bg-gray-900 border border-gray-700 rounded-lg p-3 text-white focus:border-blue-500 focus:outline-none"
                    />
                  </div>
                  <div>
                    <label className="text-xs text-gray-500 uppercase font-bold tracking-wider mb-1 block">{t('settings.region.height')}</label>
                    <input
                      type="number"
                      value={localRegion.height}
                      onChange={(e) => setLocalRegion({ ...localRegion, height: parseInt(e.target.value) || 600 })}
                      className="w-full bg-gray-900 border border-gray-700 rounded-lg p-3 text-white focus:border-blue-500 focus:outline-none"
                    />
                  </div>
                </div>

                <p className="text-xs text-gray-500 mb-2 font-bold">{t('settings.region.presets')}</p>
                <div className="grid grid-cols-3 gap-2">
                  {[
                    { label: t('settings.region.preset_720p'), w: 1280, h: 720 },
                    { label: t('settings.region.preset_1080p'), w: 1920, h: 1080 },
                    { label: t('settings.region.preset_1440p'), w: 2560, h: 1440 },
                    { label: t('settings.region.preset_4k'), w: 3840, h: 2160 },
                    { label: t('settings.region.preset_squares'), w: 1080, h: 1080 },
                    { label: t('settings.region.preset_small'), w: 800, h: 600 },
                  ].map((preset) => (
                    <button
                      key={preset.label}
                      onClick={() => {
                        const mon = monitors[localMonitor];
                        if (mon) {
                          setLocalRegion({
                            x: mon.x + Math.floor((mon.width - preset.w) / 2),
                            y: mon.y + Math.floor((mon.height - preset.h) / 2),
                            width: preset.w,
                            height: preset.h,
                          });
                        } else {
                          setLocalRegion({ ...localRegion, width: preset.w, height: preset.h });
                        }
                      }}
                      className="px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-sm text-gray-300 transition-colors"
                    >
                      {preset.label}
                    </button>
                  ))}
                </div>
              </SectionCard>

              <SectionCard title={t('settings.region.position')}>
                <div className="grid grid-cols-2 gap-4 mb-4">
                  <div>
                    <label className="text-xs text-gray-500 uppercase font-bold tracking-wider mb-1 block">{t('settings.region.x_pos')}</label>
                    <input
                      type="number"
                      value={localRegion.x}
                      onChange={(e) => {
                        setLocalRegion({ ...localRegion, x: parseInt(e.target.value) || 0 });
                        setPositionPreset("custom");
                      }}
                      className="w-full bg-gray-900 border border-gray-700 rounded-lg p-3 text-white focus:border-blue-500 focus:outline-none"
                    />
                  </div>
                  <div>
                    <label className="text-xs text-gray-500 uppercase font-bold tracking-wider mb-1 block">{t('settings.region.y_pos')}</label>
                    <input
                      type="number"
                      value={localRegion.y}
                      onChange={(e) => {
                        setLocalRegion({ ...localRegion, y: parseInt(e.target.value) || 0 });
                        setPositionPreset("custom");
                      }}
                      className="w-full bg-gray-900 border border-gray-700 rounded-lg p-3 text-white focus:border-blue-500 focus:outline-none"
                    />
                  </div>
                </div>

                <p className="text-xs text-gray-500 mb-2 font-bold">{t('settings.region.position_presets')}</p>
                <div className="grid grid-cols-3 gap-2 mb-4">
                  {[
                    { label: t('settings.region.center'), position: "center" },
                    { label: t('settings.region.top_left'), position: "top-left" },
                    { label: t('settings.region.top_right'), position: "top-right" },
                    { label: t('settings.region.bottom_left'), position: "bottom-left" },
                    { label: t('settings.region.bottom_right'), position: "bottom-right" },
                    { label: t('settings.region.top_center'), position: "top-center" },
                  ].map((preset) => (
                    <button
                      key={preset.position}
                      onClick={() => {
                        const mon = monitors[localMonitor];
                        if (mon) {
                          let x = mon.x;
                          let y = mon.y;

                          // Calculate position based on preset
                          switch (preset.position) {
                            case "center":
                              x = mon.x + Math.floor((mon.width - localRegion.width) / 2);
                              y = mon.y + Math.floor((mon.height - localRegion.height) / 2);
                              break;
                            case "top-left":
                              x = mon.x + 20;
                              y = mon.y + 20;
                              break;
                            case "top-right":
                              x = mon.x + mon.width - localRegion.width - 20;
                              y = mon.y + 20;
                              break;
                            case "bottom-left":
                              x = mon.x + 20;
                              y = mon.y + mon.height - localRegion.height - 20;
                              break;
                            case "bottom-right":
                              x = mon.x + mon.width - localRegion.width - 20;
                              y = mon.y + mon.height - localRegion.height - 20;
                              break;
                            case "top-center":
                              x = mon.x + Math.floor((mon.width - localRegion.width) / 2);
                              y = mon.y + 20;
                              break;
                          }

                          setLocalRegion({ ...localRegion, x, y });
                          setPositionPreset(preset.position);
                        }
                      }}
                      className={`px-3 py-2 rounded text-sm transition-colors ${positionPreset === preset.position
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-700 hover:bg-gray-600 text-gray-300'
                        }`}
                    >
                      {preset.label}
                    </button>
                  ))}
                </div>

                <p className="text-xs text-gray-500 mb-2 font-bold">{t('settings.region.snap_presets')} ({t('settings.region.snap_presets_desc')})</p>
                <div className="grid grid-cols-4 gap-2">
                  {[
                    { label: t('settings.region.snap_left_half'), snap: "left-half", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="18" height="20" fill="currentColor" opacity="0.8" /><rect x="20" y="2" width="18" height="20" fill="currentColor" opacity="0.2" /></svg> },
                    { label: t('settings.region.snap_right_half'), snap: "right-half", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="18" height="20" fill="currentColor" opacity="0.2" /><rect x="20" y="2" width="18" height="20" fill="currentColor" opacity="0.8" /></svg> },
                    { label: t('settings.region.snap_top_half'), snap: "top-half", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="36" height="9" fill="currentColor" opacity="0.8" /><rect x="2" y="13" width="36" height="9" fill="currentColor" opacity="0.2" /></svg> },
                    { label: t('settings.region.snap_bottom_half'), snap: "bottom-half", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="36" height="9" fill="currentColor" opacity="0.2" /><rect x="2" y="13" width="36" height="9" fill="currentColor" opacity="0.8" /></svg> },
                    { label: t('settings.region.snap_top_left_quarter'), snap: "top-left-quarter", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="18" height="9" fill="currentColor" opacity="0.8" /><rect x="20" y="2" width="18" height="9" fill="currentColor" opacity="0.2" /><rect x="2" y="13" width="18" height="9" fill="currentColor" opacity="0.2" /><rect x="20" y="13" width="18" height="9" fill="currentColor" opacity="0.2" /></svg> },
                    { label: t('settings.region.snap_top_right_quarter'), snap: "top-right-quarter", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="18" height="9" fill="currentColor" opacity="0.2" /><rect x="20" y="2" width="18" height="9" fill="currentColor" opacity="0.8" /><rect x="2" y="13" width="18" height="9" fill="currentColor" opacity="0.2" /><rect x="20" y="13" width="18" height="9" fill="currentColor" opacity="0.2" /></svg> },
                    { label: t('settings.region.snap_bottom_left_quarter'), snap: "bottom-left-quarter", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="18" height="9" fill="currentColor" opacity="0.2" /><rect x="20" y="2" width="18" height="9" fill="currentColor" opacity="0.2" /><rect x="2" y="13" width="18" height="9" fill="currentColor" opacity="0.8" /><rect x="20" y="13" width="18" height="9" fill="currentColor" opacity="0.2" /></svg> },
                    { label: t('settings.region.snap_bottom_right_quarter'), snap: "bottom-right-quarter", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="18" height="9" fill="currentColor" opacity="0.2" /><rect x="20" y="2" width="18" height="9" fill="currentColor" opacity="0.2" /><rect x="2" y="13" width="18" height="9" fill="currentColor" opacity="0.2" /><rect x="20" y="13" width="18" height="9" fill="currentColor" opacity="0.8" /></svg> },
                    { label: t('settings.region.snap_left_third'), snap: "left-third", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="11" height="20" fill="currentColor" opacity="0.8" /><rect x="15" y="2" width="11" height="20" fill="currentColor" opacity="0.2" /><rect x="28" y="2" width="10" height="20" fill="currentColor" opacity="0.2" /></svg> },
                    { label: t('settings.region.snap_right_third'), snap: "right-third", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="11" height="20" fill="currentColor" opacity="0.2" /><rect x="15" y="2" width="11" height="20" fill="currentColor" opacity="0.2" /><rect x="28" y="2" width="10" height="20" fill="currentColor" opacity="0.8" /></svg> },
                    { label: t('settings.region.snap_left_two_thirds'), snap: "left-two-thirds", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="24" height="20" fill="currentColor" opacity="0.8" /><rect x="28" y="2" width="10" height="20" fill="currentColor" opacity="0.2" /></svg> },
                    { label: t('settings.region.snap_right_two_thirds'), snap: "right-two-thirds", svg: <svg viewBox="0 0 40 24" className="w-10 h-6"><rect x="2" y="2" width="11" height="20" fill="currentColor" opacity="0.2" /><rect x="15" y="2" width="23" height="20" fill="currentColor" opacity="0.8" /></svg> },
                  ].map((preset) => (
                    <button
                      key={preset.snap}
                      onClick={() => {
                        const mon = monitors[localMonitor];
                        if (mon) {
                          let x = mon.x;
                          let y = mon.y;
                          let w = mon.width;
                          let h = mon.height;

                          // Calculate size and position based on snap preset
                          switch (preset.snap) {
                            case "left-half":
                              w = Math.floor(mon.width / 2);
                              break;
                            case "right-half":
                              x = mon.x + Math.floor(mon.width / 2);
                              w = Math.floor(mon.width / 2);
                              break;
                            case "top-half":
                              h = Math.floor(mon.height / 2);
                              break;
                            case "bottom-half":
                              y = mon.y + Math.floor(mon.height / 2);
                              h = Math.floor(mon.height / 2);
                              break;
                            case "top-left-quarter":
                              w = Math.floor(mon.width / 2);
                              h = Math.floor(mon.height / 2);
                              break;
                            case "top-right-quarter":
                              x = mon.x + Math.floor(mon.width / 2);
                              w = Math.floor(mon.width / 2);
                              h = Math.floor(mon.height / 2);
                              break;
                            case "bottom-left-quarter":
                              y = mon.y + Math.floor(mon.height / 2);
                              w = Math.floor(mon.width / 2);
                              h = Math.floor(mon.height / 2);
                              break;
                            case "bottom-right-quarter":
                              x = mon.x + Math.floor(mon.width / 2);
                              y = mon.y + Math.floor(mon.height / 2);
                              w = Math.floor(mon.width / 2);
                              h = Math.floor(mon.height / 2);
                              break;
                            case "left-third":
                              w = Math.floor(mon.width / 3);
                              break;
                            case "right-third":
                              x = mon.x + Math.floor(mon.width * 2 / 3);
                              w = Math.floor(mon.width / 3);
                              break;
                            case "left-two-thirds":
                              w = Math.floor(mon.width * 2 / 3);
                              break;
                            case "right-two-thirds":
                              x = mon.x + Math.floor(mon.width / 3);
                              w = Math.floor(mon.width * 2 / 3);
                              break;
                          }

                          setLocalRegion({ x, y, width: w, height: h });
                          setPositionPreset(preset.snap);
                        }
                      }}
                      className="flex flex-col items-center gap-1 px-2 py-2 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors group"
                    >
                      <div className="text-blue-400 group-hover:text-blue-300 transition-colors">
                        {preset.svg}
                      </div>
                      <span className="text-[10px] text-gray-300">{preset.label}</span>
                    </button>
                  ))}
                </div>
              </SectionCard>
            </div>
          )}

          {/* TAB: PERFORMANCE */}
          {activeTab === "performance" && (
            <div className="space-y-6 animate-fadeIn">
              <SectionCard title={t('settings.performance.target_framerate')} className="border-l-4 border-l-green-500">
                <div className="mb-6">
                  <div className="flex justify-between items-end mb-4">
                    <div className="text-4xl font-bold text-white">{localSettings.target_fps} <span className="text-lg text-gray-500 font-normal">FPS</span></div>
                    <div className="text-right">
                      <div className="text-sm font-medium text-gray-400">{t('settings.performance.monitor_refresh_rate')}</div>
                      <div className="text-white font-mono">{monitorRefreshRate} Hz</div>
                    </div>
                  </div>

                  <input
                    type="range"
                    min="15"
                    max={maxFps}
                    step="5"
                    value={Math.min(localSettings.target_fps, maxFps)}
                    onInput={(e) => setLocalSettings({ ...localSettings, target_fps: parseInt((e.target as HTMLInputElement).value) })}
                    onChange={(e) => setLocalSettings({ ...localSettings, target_fps: parseInt(e.target.value) })}
                    className="w-full h-3 bg-gray-700 rounded-lg cursor-pointer accent-green-500 hover:accent-green-400"
                    style={{ WebkitAppRegion: 'no-drag' } as any} onMouseDown={(e) => e.stopPropagation()}


                  />
                  <div className="flex justify-between text-xs text-gray-500 mt-2 font-mono">
                    <span>15 FPS</span>
                    <span>MAX {maxFps} FPS</span>
                  </div>
                </div>

                <div className="bg-gray-700/30 rounded p-3 text-sm text-gray-400">
                  <span className="text-green-400 font-bold">{t('settings.capture.recommended')}:</span> {t('settings.performance.fps_desc')}
                </div>
              </SectionCard>
            </div>
          )}

          {/* TAB: SHARE CONTENT */}
          {activeTab === "share_content" && (
            <WindowExclusionTab settings={localSettings} onSettingsChange={setLocalSettings} platformInfo={platformInfo} />
          )}

          {/* TAB: ADVANCED */}
          {activeTab === "advanced" && (
            <div className="space-y-6 animate-fadeIn">
              <SectionCard title={t('settings.advanced.startup_behavior')}>
                <label className="flex items-center justify-between cursor-pointer">
                  <div>
                    <span className="font-medium text-gray-200 block">{t('settings.advanced.remember_last_region')}</span>
                    <span className="text-xs text-gray-500">{t('settings.advanced.remember_last_region_desc')}</span>
                  </div>
                  <div className={`w-12 h-6 rounded-full p-1 transition-colors ${localSettings.remember_last_region ? 'bg-blue-600' : 'bg-gray-600'}`}>
                    <input
                      type="checkbox"
                      className="hidden"
                      checked={localSettings.remember_last_region}
                      onChange={(e) => setLocalSettings({ ...localSettings, remember_last_region: e.target.checked })}
                    />
                    <div className={`w-4 h-4 rounded-full bg-white transition-transform ${localSettings.remember_last_region ? 'translate-x-6' : ''}`}></div>
                  </div>
                </label>
              </SectionCard>

              <SectionCard title={t('settings.advanced.logging_troubleshooting')}>
                <div className="grid grid-cols-2 gap-4 mb-4">
                  <div>
                    <label className="block text-sm text-gray-400 mb-2">{t('settings.advanced.log_level')}</label>
                    <select
                      value={localSettings.log_level}
                      onChange={(e) => setLocalSettings({ ...localSettings, log_level: e.target.value })}
                      className="w-full h-10 bg-gray-900 border border-gray-700 rounded-lg px-3 text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    >
                      <option value="Off">{t('settings.advanced.log_level_off')}</option>
                      <option value="Error">{t('settings.advanced.log_level_error')}</option>
                      <option value="Warn">{t('settings.advanced.log_level_warn')}</option>
                      <option value="Info">{t('settings.advanced.log_level_info')}</option>
                      <option value="Debug">{t('settings.advanced.log_level_debug')}</option>
                    </select>
                  </div>

                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <label className="text-sm text-gray-400">{t('settings.advanced.log_retention')}</label>
                      <label className="flex items-center gap-2 cursor-pointer text-xs text-blue-400 hover:text-blue-300">
                        <input
                          type="checkbox"
                          checked={localSettings.log_to_file}
                          onChange={(e) => setLocalSettings({ ...localSettings, log_to_file: e.target.checked })}
                          className="rounded bg-gray-700 border-gray-600 text-blue-600 focus:ring-offset-gray-900"
                        />
                        {t('settings.advanced.log_to_file')}
                      </label>
                    </div>
                    <div className="relative">
                      <input
                        type="number"
                        min="1"
                        disabled={!localSettings.log_to_file}
                        value={localSettings.log_retention_days}
                        onChange={(e) => setLocalSettings({ ...localSettings, log_retention_days: parseInt(e.target.value) })}
                        className="w-full bg-gray-900 border border-gray-700 rounded-lg p-2 text-white disabled:opacity-50 disabled:cursor-not-allowed focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                      />
                      <span className="absolute right-3 top-2 text-gray-500 text-sm pointer-events-none">{t('settings.advanced.log_retention_days')}</span>
                    </div>
                  </div>
                </div>

                <div className="flex gap-3 mt-4">
                  <button onClick={() => invoke("open_logs_folder")} className="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded-lg text-sm text-white transition-colors">
                    {t('settings.advanced.open_log_folder')}
                  </button>
                  <button onClick={async () => {
                    const confirmed = await ask(t('settings.advanced.clear_old_logs_confirm', { days: localSettings.log_retention_days }), { title: t('settings.advanced.clear_old_logs_title'), kind: 'warning' });
                    if (confirmed) {
                      const count = (await invoke("clear_old_logs", { keepDays: localSettings.log_retention_days })) as number;
                      setToastMessage(t('settings.advanced.logs_deleted', { count }));
                    }
                  }} className="px-4 py-2 bg-red-900/50 hover:bg-red-900/80 text-red-100 rounded-lg text-sm transition-colors border border-red-800">
                    {t('settings.advanced.clear_old_logs')}
                  </button>
                </div>


              </SectionCard>
            </div>
          )}

          {/* TAB: PROFILES */}
          {activeTab === "profiles" && (
            <div className="space-y-6 animate-fadeIn">
              {/* Section 1: Profile Updates (moved to top) */}
              <SectionCard title={t('settings.profiles.profile_updates')}>
                <div className="space-y-4">
                  <p className="text-sm text-gray-400">
                    {t('settings.profiles.profile_updates_desc')}
                  </p>

                  <button
                    onClick={async () => {
                      setProfilesLoading(true);
                      try {
                        // Fetch remote version data
                        const remoteData = await invoke("check_profile_updates");

                        // Get local version data
                        let localData: any = null;
                        try {
                          localData = await invoke("get_local_profile_version");
                        } catch (error) {
                          console.warn("Local version.json missing or invalid, starting from empty.", error);
                          localData = { version: "0", last_updated: "", profiles: {} };
                        }

                        // Full sync: add/update/delete profiles
                        const platform = platformInfo.os_type || "windows";
                        const remoteProfiles = remoteData.profiles?.[platform] || {};
                        const localProfiles = localData.profiles?.[platform] || {};

                        let addedCount = 0;
                        let updatedCount = 0;
                        let deletedCount = 0;
                        let skippedCount = 0;

                        // Add or update profiles from remote
                        for (const [profileId, remoteInfo] of Object.entries(remoteProfiles)) {
                          const localInfo = localProfiles[profileId];
                          const remoteFileName = typeof (remoteInfo as any)?.file === "string"
                            ? (remoteInfo as any).file
                            : `${profileId}.json`;

                          if (!localInfo) {
                            // New profile - download it
                            try {
                              await invoke("download_profile", { profileId, fileName: remoteFileName });
                              addedCount++;
                            } catch (err) {
                              const errStr = String(err);
                              if (errStr.includes("not found") || errStr.includes("404")) {
                                skippedCount++;
                              } else {
                                console.error(`Failed to download ${profileId}:`, err);
                              }
                            }
                          } else if (localInfo.version !== remoteInfo.version) {
                            // Existing profile with different version - update it
                            try {
                              await invoke("download_profile", { profileId, fileName: remoteFileName });
                              updatedCount++;
                            } catch (err) {
                              const errStr = String(err);
                              if (errStr.includes("not found") || errStr.includes("404")) {
                                skippedCount++;
                              } else {
                                console.error(`Failed to update ${profileId}:`, err);
                              }
                            }
                          }
                        }

                        // Delete profiles that no longer exist in remote
                        for (const [profileId] of Object.entries(localProfiles)) {
                          if (!remoteProfiles[profileId]) {
                            try {
                              await invoke("delete_profile", { profileId });
                              deletedCount++;
                            } catch (err) {
                              console.error(`Failed to delete ${profileId}:`, err);
                            }
                          }
                        }

                        // Update local version.json
                        await invoke("update_local_profile_version", { versionData: remoteData });
                        setProfileVersionData(remoteData);

                        // Reload available profiles
                        const updatedProfiles = await invoke("get_capture_profiles");
                        setAvailableProfiles(updatedProfiles);

                        // Show result
                        const changes = [];
                        if (addedCount > 0) changes.push(t('settings.profiles.added_count', { count: addedCount }));
                        if (updatedCount > 0) changes.push(t('settings.profiles.updated_count', { count: updatedCount }));
                        if (deletedCount > 0) changes.push(t('settings.profiles.deleted_count', { count: deletedCount }));
                        if (skippedCount > 0) changes.push(t('settings.profiles.skipped_count', { count: skippedCount }));

                        if (changes.length > 0) {
                          setToastMessage(t('settings.profiles.sync_success', { changes: changes.join(", ") }));
                        } else {
                          setToastMessage(t('settings.profiles.all_up_to_date'));
                        }
                        setTimeout(() => setToastMessage(null), 3000);
                      } catch (error: any) {
                        console.error("Profile sync failed:", error);

                        // User-friendly error messages
                        let userMessage = t('settings.profiles.sync_failed_generic');
                        const errorStr = String(error);

                        if (errorStr.includes("Network error") || errorStr.includes("fetch")) {
                          userMessage = t('settings.profiles.sync_failed_network');
                        } else if (errorStr.includes("Data error") || errorStr.includes("parse")) {
                          userMessage = t('settings.profiles.sync_failed_data');
                        } else if (errorStr.includes("status")) {
                          userMessage = t('settings.profiles.sync_failed_server');
                        }

                        setToastMessage(userMessage);
                        setTimeout(() => setToastMessage(null), 5000);
                      } finally {
                        setProfilesLoading(false);
                      }
                    }}
                    disabled={profilesLoading}
                    className="w-full px-4 py-3 bg-green-600 hover:bg-green-700 disabled:bg-gray-700 disabled:cursor-not-allowed text-white rounded-lg transition-colors flex items-center justify-center gap-2"
                  >
                    {profilesLoading ? (
                      <>
                        <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                        </svg>
                        {t('app.loading')}
                      </>
                    ) : (
                      <>
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                        {t('settings.share_content.refresh')}
                      </>
                    )}
                  </button>

                  {profileVersionData && (
                    <div className="p-3 bg-green-500/10 border border-green-500/20 rounded-lg text-sm">
                      <div className="flex items-center gap-2 text-green-400">
                        <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                          <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                        </svg>
                        <span>{t('settings.profiles.last_synced', { version: profileVersionData.version, date: profileVersionData.last_updated })}</span>
                      </div>
                    </div>
                  )}
                </div>
              </SectionCard>

              <div className="bg-purple-500/10 border border-purple-500/20 rounded-xl p-4">
                <div className="flex items-center gap-3 mb-2">
                  <svg className="w-6 h-6 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                  </svg>
                  <div className="flex-1">
                    <h3 className="font-bold text-white">{t('settings.profiles.capture_profiles_title')}</h3>
                    <p className="text-sm text-gray-400">{t('settings.profiles.capture_profiles_desc')}</p>
                  </div>
                  <button
                    onClick={() => open("https://github.com/salihcantekin/RustFrame/tree/master/resources/profiles")}
                    className="text-blue-400 hover:text-blue-300 text-sm flex items-center gap-1"
                  >
                    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                      <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                    </svg>
                    {t('settings.profiles.view_on_github')}
                  </button>
                </div>
              </div>

              {/* Section 2: Local Profile Management */}
              <SectionCard title={t('settings.profiles.local_profiles')}>
                <div className="space-y-4">
                  <p className="text-sm text-gray-400">
                    {t('settings.profiles.local_profiles_desc')}
                  </p>

                  <div>
                    <label className="text-sm text-gray-400 block mb-2">{t('settings.profiles.select_profile')}</label>
                    <select
                      value={selectedProfileForDetails}
                      onChange={async (e) => {
                        const profileId = e.target.value;
                        setSelectedProfileForDetails(profileId);
                        if (profileId) {
                          try {
                            const details = await invoke("get_profile_details", { profileId });
                            setProfileDetails(details);
                          } catch (error) {
                            console.error("Failed to load profile details:", error);
                            setProfileDetails(null);
                          }
                        } else {
                          setProfileDetails(null);
                        }
                      }}
                      className="w-full px-4 py-3 bg-gray-900 border border-gray-700 rounded-lg focus:ring-2 focus:ring-blue-500 focus:outline-none text-white"
                    >
                      <option value="">-- {t('settings.profiles.select_profile_placeholder')} --</option>
                      {availableProfiles.map((profile) => (
                        <option key={profile.id} value={profile.id}>
                          {profile.name || profile.id}
                        </option>
                      ))}
                    </select>
                  </div>

                  {selectedProfileForDetails && (
                    <button
                      onClick={async () => {
                        if (await ask(t('settings.profiles.delete_profile_confirm', { profileId: selectedProfileForDetails }), { kind: "warning" })) {
                          try {
                            await invoke("delete_profile", { profileId: selectedProfileForDetails });
                            setToastMessage(t('settings.profiles.profile_deleted_success'));
                            setSelectedProfileForDetails("");
                            setProfileDetails(null);

                            // Reload profiles
                            const updatedProfiles = await invoke("get_capture_profiles");
                            setAvailableProfiles(updatedProfiles);

                            setTimeout(() => setToastMessage(null), 3000);
                          } catch (error) {
                            console.error("Delete failed:", error);
                            setToastMessage(t('settings.profiles.delete_profile_failed'));
                            setTimeout(() => setToastMessage(null), 5000);
                          }
                        }
                      }}
                      className="w-full px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors flex items-center justify-center gap-2"
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                      {t('settings.profiles.delete_profile')}
                    </button>
                  )}
                </div>
              </SectionCard>

              {profileDetails && (
                <>
                  <SectionCard title={profileDetails.name}>
                    <div className="space-y-4">
                      <p className="text-gray-300">{profileDetails.description}</p>

                      {profileDetails.settings.explanation && (
                        <div className="p-4 bg-blue-500/10 border border-blue-500/20 rounded-lg">
                          <div className="flex items-start gap-2">
                            <svg className="w-5 h-5 text-blue-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                            </svg>
                            <div>
                              <h4 className="font-bold text-blue-300 mb-1">{t('settings.profiles.why_these_settings')}</h4>
                              <p className="text-sm text-gray-300 leading-relaxed">{profileDetails.settings.explanation}</p>
                            </div>
                          </div>
                        </div>
                      )}

                      <div className="space-y-2">
                        <h4 className="font-bold text-white text-sm">{t('settings.profiles.profile_settings')}:</h4>
                        <div className="bg-gray-900/50 rounded-lg border border-gray-700 divide-y divide-gray-800">
                          {Object.entries(profileDetails.settings).map(([key, value]: [string, any]) => {
                            if (key === "name" || key === "description" || key === "explanation") return null;

                            const tooltips: { [k: string]: string } = {
                              "winapi_destination_overlapped": "WS_OVERLAPPEDWINDOW: Makes window appear as a standard application window with title bar and borders",
                              "winapi_destination_appwindow": "WS_EX_APPWINDOW: Forces window to appear in taskbar and Alt-Tab switcher",
                              "winapi_destination_toolwindow": "WS_EX_TOOLWINDOW: Hides window from taskbar, appears as a tool window",
                              "winapi_destination_layered": "WS_EX_LAYERED: Enables transparency and alpha blending support",
                              "winapi_destination_alpha": "Window opacity level (0-255). 255 = fully opaque",
                              "winapi_destination_topmost": "WS_EX_TOPMOST: Keeps window above all non-topmost windows",
                              "winapi_destination_click_through": "WS_EX_TRANSPARENT: Makes window click-through (mouse events pass to windows below)",
                              "winapi_destination_noactivate": "WS_EX_NOACTIVATE: Prevents window from stealing focus when clicked",
                              "winapi_destination_hide_taskbar_after_ms": "Milliseconds to wait before hiding window from taskbar (null = never hide)"
                            };

                            return (
                              <div key={key} className="flex items-center justify-between p-3 group hover:bg-gray-800/50 transition-colors">
                                <div className="flex items-center gap-2 flex-1">
                                  <span className="text-gray-400 text-sm font-mono">{key}</span>
                                  {tooltips[key] && (
                                    <div className="relative group/tooltip">
                                      <svg className="w-4 h-4 text-gray-600 hover:text-blue-400 cursor-help" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                                      </svg>
                                      <div className="absolute left-0 top-6 w-64 p-2 bg-gray-950 border border-gray-700 rounded-lg shadow-xl z-50 opacity-0 group-hover/tooltip:opacity-100 pointer-events-none transition-opacity text-xs text-gray-300">
                                        {tooltips[key]}
                                      </div>
                                    </div>
                                  )}
                                </div>
                                <span className="text-white font-mono text-sm font-semibold">{JSON.stringify(value)}</span>
                              </div>
                            );
                          })}
                        </div>
                      </div>
                    </div>
                  </SectionCard>
                </>
              )}
            </div>
          )}

          {/* TAB: ABOUT */}
          {activeTab === "about" && (
            <div className="space-y-6 animate-fadeIn">
              <div className="text-center py-8">
                <div className="w-20 h-20 bg-gradient-to-br from-blue-500 to-purple-600 rounded-2xl mx-auto flex items-center justify-center shadow-lg mb-4">
                  <img src="/icon.png" className="w-12 h-12 drop-shadow-md" alt="Logo" />
                </div>
                <h2 className="text-3xl font-bold text-white mb-2">RustFrame</h2>
                <p className="text-gray-400">{t('settings.about.tagline')}</p>
                <p className="text-gray-500 text-sm mt-2">v{appVersion} • {platformInfo.os_type} {platformInfo.os_version !== "Unknown" ? platformInfo.os_version : ""}</p>
              </div>

              <div className="bg-gray-800/50 rounded-xl p-4 text-center">
                <button onClick={() => invoke("open_settings_folder")} className="text-blue-400 hover:text-blue-300 hover:underline text-sm">
                  {t('settings.about.config_folder')}
                </button>
              </div>
            </div>
          )}

        </div>

        {/* Footer Actions */}
        <div className="px-6 py-4 border-t border-gray-800 bg-gray-900/80 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <button
              onClick={handleExportSettings}
              className="px-4 py-2 text-sm font-medium text-gray-300 hover:text-white hover:bg-gray-800 rounded-xl transition-all"
            >
              {t('settings.export')}
            </button>
            <button
              onClick={handleImportSettings}
              className="px-4 py-2 text-sm font-medium text-gray-300 hover:text-white hover:bg-gray-800 rounded-xl transition-all"
            >
              {t('settings.import')}
            </button>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={onClose}
              className="px-6 py-2 text-sm font-medium text-gray-400 hover:text-white transition-colors"
            >
              {t('app.close')}
            </button>
            <button
              onClick={handleSave}
              className="px-8 py-2.5 bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-500 hover:to-blue-600 text-white rounded-xl font-bold shadow-lg shadow-blue-500/20 transform hover:scale-[1.02] active:scale-95 transition-all"
            >
              {t('settings.save')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default SettingsDialog;
