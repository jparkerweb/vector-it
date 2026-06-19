import { useEffect, useCallback } from "react";

export type ViewMode = "original" | "segmentation" | "vector";

interface ViewToggleProps {
  activeView: ViewMode;
  onViewChange: (view: ViewMode) => void;
}

const VIEW_OPTIONS: { key: ViewMode; label: string; shortcut: string }[] = [
  { key: "original", label: "Original", shortcut: "1" },
  { key: "segmentation", label: "Segmentation", shortcut: "2" },
  { key: "vector", label: "Vector", shortcut: "3" },
];

export function ViewToggle({ activeView, onViewChange }: ViewToggleProps) {
  // Keyboard shortcuts
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      // Don't capture if user is typing in an input
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return;
      }

      switch (e.key) {
        case "1":
          onViewChange("original");
          break;
        case "2":
          onViewChange("segmentation");
          break;
        case "3":
          onViewChange("vector");
          break;
      }
    },
    [onViewChange]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="flex items-center bg-white/90 backdrop-blur rounded-lg shadow px-1 py-1 gap-0.5">
      {VIEW_OPTIONS.map(({ key, label, shortcut }) => (
        <button
          key={key}
          onClick={() => onViewChange(key)}
          className={`px-3 py-1.5 text-xs font-medium rounded transition-all duration-200 ${
            activeView === key
              ? "bg-blue-500 text-white shadow-sm"
              : "text-gray-600 hover:text-gray-900 hover:bg-gray-100"
          }`}
          title={`${label} (${shortcut})`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
