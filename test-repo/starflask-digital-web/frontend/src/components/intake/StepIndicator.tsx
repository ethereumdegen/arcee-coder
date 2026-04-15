const stepLabels = ['Type', 'Details', 'Upload', 'Contact', 'Review']

interface StepIndicatorProps {
  currentStep: number
  totalSteps: number
}

export default function StepIndicator({ currentStep, totalSteps }: StepIndicatorProps) {
  return (
    <div className="flex items-center justify-center gap-2 mb-10">
      {Array.from({ length: totalSteps }).map((_, i) => (
        <div key={i} className="flex items-center">
          <div className="flex flex-col items-center">
            <div
              className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-semibold transition-all duration-300
                ${i <= currentStep
                  ? 'bg-accent text-white'
                  : 'bg-gray-200 text-gray-400'
                }
                ${i === currentStep ? 'ring-4 ring-accent/30 scale-110' : ''}
              `}
            >
              {i < currentStep ? (
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                </svg>
              ) : (
                i + 1
              )}
            </div>
            <span className="text-xs mt-1 text-gray-500 hidden sm:block">{stepLabels[i]}</span>
          </div>
          {i < totalSteps - 1 && (
            <div className="w-8 sm:w-12 h-0.5 bg-gray-200 mx-1">
              <div
                className="h-full bg-accent transition-all duration-500"
                style={{ width: i < currentStep ? '100%' : '0%' }}
              />
            </div>
          )}
        </div>
      ))}
    </div>
  )
}
