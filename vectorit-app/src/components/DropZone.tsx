import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore } from "../stores/appStore";

export function DropZone() {
  const { loadImage, vectorize, isProcessing, imageInfo } = useAppStore();
  const [isDragOver, setIsDragOver] = useState(false);

  const handleOpen = useCallback(async () => {
    const selected = await open({
      multiple: false,
      filters: [
        { name: "Images", extensions: ["png", "jpg", "jpeg", "bmp", "gif", "tiff", "tif"] },
      ],
    });
    if (selected) {
      await loadImage(selected);
      await vectorize();
    }
  }, [loadImage, vectorize]);

  // Tauri 2 drag-and-drop: listen for native file drop events
  useEffect(() => {
    const unlisten = getCurrentWindow().onDragDropEvent(async (event) => {
      if (event.payload.type === "enter" || event.payload.type === "over") {
        setIsDragOver(true);
      } else if (event.payload.type === "leave") {
        setIsDragOver(false);
      } else if (event.payload.type === "drop") {
        setIsDragOver(false);
        const paths = event.payload.paths;
        if (paths.length > 0) {
          await loadImage(paths[0]);
          await vectorize();
        }
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, [loadImage, vectorize]);

  if (imageInfo) return null;

  return (
    <div
      className={`flex flex-col items-center justify-center h-full border-2 border-dashed rounded-lg p-8 cursor-pointer transition-colors ${
        isDragOver ? "border-blue-500 bg-blue-50" : "border-gray-400 hover:border-blue-500"
      }`}
      onClick={handleOpen}
    >
      {isProcessing ? (
        <div className="text-gray-600">Processing...</div>
      ) : (
        <>
          <svg
            className="w-16 h-16 text-gray-400 mb-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
            />
          </svg>
          <p className="text-gray-600 text-lg font-medium">
            Drop an image here or click to browse
          </p>
          <p className="text-gray-400 text-sm mt-2">
            Supports PNG, JPG, BMP, GIF, TIFF
          </p>
        </>
      )}
    </div>
  );
}
