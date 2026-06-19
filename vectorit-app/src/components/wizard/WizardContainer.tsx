import { useWizardStore, WizardStep } from "../../stores/wizardStore";
import { ImageTypeStep } from "./ImageTypeStep";
import { QualityStep } from "./QualityStep";
import { ColorModeStep } from "./ColorModeStep";
import { ReviewStep } from "./ReviewStep";
import { ExportStep } from "./ExportStep";

const STEP_LABELS: Record<WizardStep, string> = {
  imageType: "Image Type",
  quality: "Quality",
  colorMode: "Colors",
  review: "Review",
  export: "Export",
};

export function WizardContainer() {
  const { currentStep, steps, nextStep, prevStep } = useWizardStore();
  const currentIdx = steps.indexOf(currentStep);
  const isFirst = currentIdx === 0;
  const isLast = currentIdx === steps.length - 1;

  return (
    <div className="flex flex-col h-full">
      {/* Step indicator */}
      <div className="flex items-center gap-1 p-4 border-b border-gray-200 bg-gray-50">
        {steps.map((step, i) => (
          <div key={step} className="flex items-center">
            {i > 0 && <div className="w-6 h-px bg-gray-300 mx-1" />}
            <div
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium transition-colors ${
                step === currentStep
                  ? "bg-blue-600 text-white"
                  : i < currentIdx
                  ? "bg-blue-100 text-blue-700"
                  : "bg-gray-200 text-gray-500"
              }`}
            >
              <span className="w-4 h-4 rounded-full flex items-center justify-center text-[10px] bg-white/20">
                {i < currentIdx ? "✓" : i + 1}
              </span>
              {STEP_LABELS[step]}
            </div>
          </div>
        ))}
      </div>

      {/* Step content */}
      <div className="flex-1 overflow-auto p-6">
        {currentStep === "imageType" && <ImageTypeStep />}
        {currentStep === "quality" && <QualityStep />}
        {currentStep === "colorMode" && <ColorModeStep />}
        {currentStep === "review" && <ReviewStep />}
        {currentStep === "export" && <ExportStep />}
      </div>

      {/* Navigation buttons */}
      <div className="flex items-center justify-between p-4 border-t border-gray-200 bg-gray-50">
        <button
          onClick={prevStep}
          disabled={isFirst}
          className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed"
        >
          ← Back
        </button>
        {!isLast && (
          <button
            onClick={nextStep}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded hover:bg-blue-700"
          >
            Next →
          </button>
        )}
      </div>
    </div>
  );
}
