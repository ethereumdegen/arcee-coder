export type ProjectType = 'inbound_sales' | 'web_design' | 'agentic_automation' | 'multiple'

export type BudgetRange = '$250-750' | '$750-2K' | '$2K-5K' | '$5K+'

export type Timeline = 'asap' | '1-2 months' | '3-6 months' | 'flexible'

export type SubmissionStatus = 'new' | 'reviewed' | 'contacted' | 'closed'

export interface UploadedFile {
  storage_key: string
  url: string
  filename: string
  file_type: string
  file_size: number
}

export interface IntakeFormData {
  project_type: ProjectType
  title: string
  description: string
  budget_range: BudgetRange
  timeline: Timeline
  full_name: string
  email: string
  phone?: string
  company?: string
  referral_source?: string
  files: UploadedFile[]
}

export interface Submission extends IntakeFormData {
  id: string
  status: SubmissionStatus
  notes: string | null
  created_at: string
  updated_at: string
}

export interface SubmissionFile extends UploadedFile {
  id: string
  submission_id: string
  created_at: string
}
