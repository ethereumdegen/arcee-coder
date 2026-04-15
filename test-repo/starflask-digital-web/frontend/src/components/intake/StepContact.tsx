import type { UseFormRegister, FieldErrors } from 'react-hook-form'
import type { SubmissionInput } from '../../lib/validations'
import Input from '../ui/Input'

interface StepContactProps {
  register: UseFormRegister<SubmissionInput>
  errors: FieldErrors<SubmissionInput>
}

export default function StepContact({ register, errors }: StepContactProps) {
  return (
    <div>
      <h3 className="text-2xl font-heading font-bold text-gray-900 mb-2">Contact Info</h3>
      <p className="text-gray-500 mb-8">How can we reach you?</p>

      <div className="space-y-5">
        <Input
          label="Full Name *"
          placeholder="Your name"
          error={errors.full_name?.message}
          {...register('full_name')}
        />

        <Input
          label="Email *"
          type="email"
          placeholder="you@company.com"
          error={errors.email?.message}
          {...register('email')}
        />

        <div className="grid sm:grid-cols-2 gap-5">
          <Input
            label="Phone"
            type="tel"
            placeholder="(555) 123-4567"
            {...register('phone')}
          />

          <Input
            label="Company"
            placeholder="Your company name"
            {...register('company')}
          />
        </div>

        <div className="space-y-1">
          <label className="block text-sm font-medium text-gray-600">How did you find us?</label>
          <select
            className="w-full px-4 py-3 rounded-xl border border-gray-200 bg-white text-gray-800
              focus:outline-none focus:ring-2 focus:ring-accent/50 focus:border-accent"
            {...register('referral_source')}
          >
            <option value="">Select...</option>
            <option value="google">Google Search</option>
            <option value="social">Social Media</option>
            <option value="referral">Referral</option>
            <option value="other">Other</option>
          </select>
        </div>
      </div>
    </div>
  )
}
