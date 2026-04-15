import { z } from 'zod'

export const fileSchema = z.object({
  storage_key: z.string(),
  url: z.string().url(),
  filename: z.string(),
  file_type: z.string(),
  file_size: z.number(),
})

export const submissionSchema = z.object({
  project_type: z.string().min(1, 'Please select a project type'),
  title: z.string().min(3, 'Title must be at least 3 characters').max(100),
  description: z.string().min(20, 'Description must be at least 20 characters'),
  budget_range: z.string().min(1, 'Please select a budget range'),
  timeline: z.string().min(1, 'Please select a timeline'),
  full_name: z.string().min(2, 'Name must be at least 2 characters').max(100),
  email: z.string().email('Please enter a valid email'),
  phone: z.string().max(30).optional().or(z.literal('')),
  company: z.string().max(100).optional().or(z.literal('')),
  referral_source: z.string().max(50).optional().or(z.literal('')),
  files: z.array(fileSchema).max(5).optional(),
})

export type SubmissionInput = z.infer<typeof submissionSchema>

export const loginSchema = z.object({
  password: z.string().min(1, 'Password is required'),
})
