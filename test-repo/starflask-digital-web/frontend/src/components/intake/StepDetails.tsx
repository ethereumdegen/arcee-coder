import type { UseFormRegister, FieldErrors } from 'react-hook-form'
import type { SubmissionInput } from '../../lib/validations'
import Input from '../ui/Input'
import Textarea from '../ui/Textarea'
import { BUDGET_LABELS, TIMELINE_LABELS } from '../../lib/constants'

interface StepDetailsProps {
  register: UseFormRegister<SubmissionInput>
  errors: FieldErrors<SubmissionInput>
  budgetRanges: string[]
  timelines: string[]
}

export default function StepDetails({ register, errors, budgetRanges, timelines }: StepDetailsProps) {
  return (
    <div>
      <h3 className="text-2xl font-heading font-bold text-gray-900 mb-2">Project Details</h3>
      <p className="text-gray-500 mb-8">Tell us about what you're building.</p>

      <div className="space-y-5">
        <Input
          label="Project Title"
          placeholder="e.g., AI-Powered Lead Scoring System"
          error={errors.title?.message}
          {...register('title')}
        />

        <Textarea
          label="Description"
          placeholder="Describe your project, goals, and any specific requirements (min 20 characters)..."
          error={errors.description?.message}
          {...register('description')}
        />

        <div className="grid sm:grid-cols-2 gap-5">
          <div className="space-y-1">
            <label className="block text-sm font-medium text-gray-600">Budget Range</label>
            <select
              className="w-full px-4 py-3 rounded-xl border border-gray-200 bg-white text-gray-800
                focus:outline-none focus:ring-2 focus:ring-accent/50 focus:border-accent"
              {...register('budget_range')}
            >
              {budgetRanges.map((b) => (
                <option key={b} value={b}>{BUDGET_LABELS[b] || b}</option>
              ))}
            </select>
          </div>

          <div className="space-y-1">
            <label className="block text-sm font-medium text-gray-600">Timeline</label>
            <select
              className="w-full px-4 py-3 rounded-xl border border-gray-200 bg-white text-gray-800
                focus:outline-none focus:ring-2 focus:ring-accent/50 focus:border-accent"
              {...register('timeline')}
            >
              {timelines.map((t) => (
                <option key={t} value={t}>{TIMELINE_LABELS[t] || t}</option>
              ))}
            </select>
          </div>
        </div>
      </div>
    </div>
  )
}
