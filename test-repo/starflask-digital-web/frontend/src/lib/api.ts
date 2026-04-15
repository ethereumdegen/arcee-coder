const API_BASE = '/api'

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    ...options,
  })

  if (!res.ok) {
    const error = await res.json().catch(() => ({ message: 'Request failed' }))
    throw new Error(error.message || `HTTP ${res.status}`)
  }

  return res.json()
}

export interface BlogPost {
  id: string
  slug: string
  title: string
  summary: string
  content: string
  category: string
  cover_gradient: string
  published: boolean
  created_at: string
  updated_at: string
}

export interface IntakeOptions {
  project_types: string[]
  budget_ranges: string[]
  timelines: string[]
  uploads_enabled: boolean
}

export const api = {
  getIntakeOptions: () =>
    request<IntakeOptions>('/intake-options'),

  submitIntake: (data: unknown) =>
    request<{ success: boolean; id: string }>('/submissions', {
      method: 'POST',
      body: JSON.stringify(data),
    }),

  adminLogin: (password: string) =>
    request<{ success: boolean }>('/admin/login', {
      method: 'POST',
      body: JSON.stringify({ password }),
    }),

  adminLogout: () =>
    request<{ success: boolean }>('/admin/logout', { method: 'POST' }),

  getSubmissions: (params?: Record<string, string>) => {
    const query = params ? '?' + new URLSearchParams(params).toString() : ''
    return request<{ submissions: unknown[] }>(`/admin/submissions${query}`)
  },

  getSubmission: (id: string) =>
    request<{ submission: unknown }>(`/admin/submissions/${id}`),

  updateSubmission: (id: string, data: unknown) =>
    request<{ success: boolean }>(`/admin/submissions/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    }),

  uploadFile: async (file: File) => {
    const formData = new FormData()
    formData.append('file', file)

    const res = await fetch('/api/upload', {
      method: 'POST',
      body: formData,
    })

    if (!res.ok) {
      throw new Error(`Upload failed: HTTP ${res.status}`)
    }

    return res.json() as Promise<{
      storage_key: string
      url: string
      filename: string
      file_type: string
      file_size: number
    }>
  },

  getBlogPosts: () =>
    request<{ posts: BlogPost[] }>('/blog'),

  getBlogPost: (slug: string) =>
    request<{ post: BlogPost }>(`/blog/${slug}`),
}
