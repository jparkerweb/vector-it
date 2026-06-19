import { useEffect } from "react";
import { useWizardStore, ImageTypeChoice } from "../../stores/wizardStore";
import { useAppStore } from "../../stores/appStore";

const IMAGE_TYPE_OPTIONS: { value: ImageTypeChoice; label: string; description: string }[] = [
  { value: "photo", label: "Photo", description: "Photographs and natural images with continuous tones" },
  { value: "logo_smooth", label: "Logo with smoothing", description: "Logos and icons with anti-aliased edges" },
  { value: "logo_sharp", label: "Logo without smoothing", description: "Pixel-art, sharp-edged graphics, aliased logos" },
];

export function ImageTypeStep() {
  const { detectedType, selectedImageType, setImageType, detectImageType } = useWizardStore();
  const { imagePath } = useAppStore();

  useEffect(() => {
    if (imagePath && !detectedType) {
      detectImageType(imagePath);
    }
  }, [imagePath, detectedType, detectImageType]);

  return (
    <div className="max-w-lg mx-auto">
      <h2 className="text-lg font-semibold text-gray-900 mb-2">Image Type</h2>
      {detectedType && (
        <p className="text-sm text-gray-500 mb-4">
          Auto-detected:{" "}
          <span className="font-medium text-gray-700">
            {detectedType.image_type}
          </span>{" "}
          ({Math.round(detectedType.confidence * 100)}% confidence)
        </p>
      )}
      <div className="space-y-3">
        {IMAGE_TYPE_OPTIONS.map((opt) => (
          <label
            key={opt.value}
            className={`flex items-start gap-3 p-4 border rounded-lg cursor-pointer transition-colors ${
              selectedImageType === opt.value
                ? "border-blue-500 bg-blue-50"
                : "border-gray-200 hover:border-gray-300"
            }`}
          >
            <input
              type="radio"
              name="imageType"
              checked={selectedImageType === opt.value}
              onChange={() => setImageType(opt.value)}
              className="mt-0.5"
            />
            <div>
              <div className="font-medium text-gray-900">{opt.label}</div>
              <div className="text-sm text-gray-500">{opt.description}</div>
            </div>
          </label>
        ))}
      </div>
    </div>
  );
}
