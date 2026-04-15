import type { IntakeFormData } from '../../types'
import { PROJECT_TYPE_LABELS, BUDGET_LABELS, TIMELINE_LABELS } from '../../lib/constants'

interface StepReviewProps {
  data: IntakeFormData
  onEdit: (step: number) => void
}

export default function StepReview({ data, onEdit }: StepReviewProps) {
  return (
    <div>
      <h3 className="text-2xl font-heading font-bold text-gray-900 mb-2">Review & Submit</h3>
      <p className="text-gray-500 mb-8">Double-check everything looks good.</p>

      <div className="space-y-6">
        <ReviewSection title="Project Type" onEdit={() => onEdit(0)}>
          <p className="text-gray-800">{PROJECT_TYPE_LABELS[data.project_type] || data.project_type}</p>
        </ReviewSection>

        <ReviewSection title="Project Details" onEdit={() => onEdit(1)}>
          <p className="text-gray-800 font-medium">{data.title}</p>
          <p className="text-gray-600 mt-1">{data.description}</p>
          <div className="flex gap-4 mt-2 text-sm text-gray-500">
            <span>Budget: {BUDGET_LABELS[data.budget_range]}</span>
            <span>Timeline: {TIMELINE_LABELS[data.timeline]}</span>
          </div>
        </ReviewSection>

        <ReviewSection title="Files" onEdit={() => onEdit(2)}>
          {data.files.length > 0 ? (
            <div className="flex gap-2 flex-wrap">
              {data.files.map((f) => (
                <span key={f.storage_key} className="px-3 py-1 bg-gray-100 rounded-lg text-sm text-gray-600">
                  {f.filename}
                </span>
              ))}
            </div>
          ) : (
            <p className="text-gray-400 italic">No files uploaded</p>
          )}
        </ReviewSection>

        <ReviewSection title="Contact" onEdit={() => onEdit(3)}>
          <p className="text-gray-800">{data.full_name}</p>
          <p className="text-gray-600">{data.email}</p>
          {data.phone && <p className="text-gray-500 text-sm">{data.phone}</p>}
          {data.company && <p className="text-gray-500 text-sm">{data.company}</p>}
        </ReviewSection>
      </div>
    </div>
  )
}

function ReviewSection({
  title,
  onEdit,
  children,
}: {
  title: string
  onEdit: () => void
  children: React.ReactNode
}) {
  return (
    <div className="p-5 rounded-xl border border-gray-200 bg-gray-50">
      <div className="flex items-center justify-between mb-3">
        <h4 className="font-heading font-semibold text-gray-700">{title}</h4>
        <button
          type="button"
          onClick={onEdit}
          className="text-sm text-accent hover:text-accent-dark font-medium cursor-pointer"
        >
          Edit
        </button>
      </div>
      {children}
    </div>
  )
}
