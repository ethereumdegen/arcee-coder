import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { toast } from 'sonner'
import { AnimatePresence, motion } from 'framer-motion'
import type { IntakeFormData, ProjectType, UploadedFile } from '../../types'
import { submissionSchema, type SubmissionInput } from '../../lib/validations'
import { api, type IntakeOptions } from '../../lib/api'
import { useMultiStepForm } from '../../hooks/useMultiStepForm'
import StepIndicator from './StepIndicator'
import StepProjectType from './StepProjectType'
import StepDetails from './StepDetails'
import StepUpload from './StepUpload'
import StepContact from './StepContact'
import StepReview from './StepReview'
import Button from '../ui/Button'

export default function IntakeWizard() {
  const navigate = useNavigate()
  const [submitting, setSubmitting] = useState(false)
  const [files, setFiles] = useState<UploadedFile[]>([])
  const [options, setOptions] = useState<IntakeOptions | null>(null)
  const { currentStep, totalSteps, isFirst, isLast, next, prev, goTo } = useMultiStepForm(5)

  useEffect(() => {
    api.getIntakeOptions().then(setOptions).catch(() => {
      toast.error('Failed to load form options')
    })
  }, [])

  const {
    register,
    handleSubmit,
    watch,
    setValue,
    getValues,
    trigger,
    formState: { errors },
  } = useForm<SubmissionInput>({
    resolver: zodResolver(submissionSchema),
    defaultValues: {
      project_type: undefined,
      title: '',
      description: '',
      budget_range: '',
      timeline: '',
      full_name: '',
      email: '',
      phone: '',
      company: '',
      referral_source: '',
      files: [],
    },
    mode: 'onBlur',
  })

  // Set defaults once options load
  useEffect(() => {
    if (options) {
      const current = getValues()
      if (!current.budget_range && options.budget_ranges.length)
        setValue('budget_range', options.budget_ranges[0])
      if (!current.timeline && options.timelines.length)
        setValue('timeline', options.timelines[0])
    }
  }, [options, setValue, getValues])

  const projectType = watch('project_type')

  const validateStep = async (): Promise<boolean> => {
    switch (currentStep) {
      case 0:
        return !!projectType
      case 1:
        return trigger(['title', 'description', 'budget_range', 'timeline'])
      case 2:
        return true
      case 3:
        return trigger(['full_name', 'email'])
      default:
        return true
    }
  }

  const handleNext = async () => {
    const valid = await validateStep()
    if (!valid) {
      if (currentStep === 0) toast.error('Please select a project type')
      return
    }
    next()
  }

  const onSubmit = async (data: SubmissionInput) => {
    setSubmitting(true)
    try {
      await api.submitIntake({ ...data, files })
      navigate('/intake/success')
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Submission failed')
    } finally {
      setSubmitting(false)
    }
  }

  if (!options) {
    return <div className="text-center py-12 text-gray-500">Loading...</div>
  }

  const formValues = getValues()

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="max-w-2xl mx-auto">
      <StepIndicator currentStep={currentStep} totalSteps={totalSteps} />

      <AnimatePresence mode="wait">
        <motion.div
          key={currentStep}
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          exit={{ opacity: 0, x: -20 }}
          transition={{ duration: 0.2 }}
        >
          {currentStep === 0 && (
            <StepProjectType
              value={(projectType || '') as ProjectType | ''}
              onChange={(val) => setValue('project_type', val)}
            />
          )}
          {currentStep === 1 && (
            <StepDetails
              register={register}
              errors={errors}
              budgetRanges={options.budget_ranges}
              timelines={options.timelines}
            />
          )}
          {currentStep === 2 && <StepUpload files={files} onFilesChange={setFiles} uploadsEnabled={options.uploads_enabled} />}
          {currentStep === 3 && <StepContact register={register} errors={errors} />}
          {currentStep === 4 && (
            <StepReview
              data={{
                ...formValues,
                project_type: formValues.project_type || 'multiple',
                files,
              } as IntakeFormData}
              onEdit={goTo}
            />
          )}
        </motion.div>
      </AnimatePresence>

      <div className="flex justify-between mt-10">
        {!isFirst ? (
          <Button type="button" variant="dark" onClick={prev}>
            Back
          </Button>
        ) : (
          <div />
        )}

        {isLast ? (
          <Button type="submit" variant="filled" loading={submitting}>
            Submit Project
          </Button>
        ) : (
          <Button type="button" variant="filled" onClick={handleNext}>
            Continue
          </Button>
        )}
      </div>
    </form>
  )
}
