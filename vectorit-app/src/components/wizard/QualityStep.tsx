import { useWizardStore, QualityChoice } from "../../stores/wizardStore";

const QUALITY_OPTIONS: { value: QualityChoice; label: string; description: string; time: string }[] = [
  {
    value: "Low",
    label: "Low",
    description: "Fewer nodes, faster processing. Good for simple shapes.",
    time: "~1s",
  },
  {
    value: "Medium",
    label: "Medium",
    description: "Balanced quality and speed. Recommended for most images.",
    time: "~3s",
  },
  {
    value: "High",
    label: "High",
    description: "Maximum detail, more nodes. Best for complex images.",
    time: "~5s",
  },
];

export function QualityStep() {
  const { selectedQuality, setQuality } = useWizardStore();

  return (
    <div className="max-w-lg mx-auto">
      <h2 className="text-lg font-semibold text-gray-900 mb-4">Quality Level</h2>
      <div className="space-y-3">
        {QUALITY_OPTIONS.map((opt) => (
          <label
            key={opt.value}
            className={`flex items-start gap-3 p-4 border rounded-lg cursor-pointer transition-colors ${
              selectedQuality === opt.value
                ? "border-blue-500 bg-blue-50"
                : "border-gray-200 hover:border-gray-300"
            }`}
          >
            <input
              type="radio"
              name="quality"
              checked={selectedQuality === opt.value}
              onChange={() => setQuality(opt.value)}
              className="mt-0.5"
            />
            <div className="flex-1">
              <div className="flex items-center justify-between">
                <span className="font-medium text-gray-900">{opt.label}</span>
                <span className="text-xs text-gray-400">{opt.time}</span>
              </div>
              <div className="text-sm text-gray-500">{opt.description}</div>
            </div>
          </label>
        ))}
      </div>
    </div>
  );
}
