import { useCallback, useState } from 'react'
import { useDropzone } from 'react-dropzone'
import type { UploadedFile } from '../../types'
import { api } from '../../lib/api'

interface FileDropzoneProps {
  files: UploadedFile[]
  onFilesChange: (files: UploadedFile[]) => void
  maxFiles?: number
}

export default function FileDropzone({ files, onFilesChange, maxFiles = 5 }: FileDropzoneProps) {
  const [uploading, setUploading] = useState(false)
  const [progress, setProgress] = useState<Record<string, number>>({})

  const onDrop = useCallback(
    async (accepted: File[]) => {
      if (files.length + accepted.length > maxFiles) return
      setUploading(true)

      try {
        const uploaded = await Promise.all(
          accepted.map(async (file) => {
            setProgress((p) => ({ ...p, [file.name]: 0 }))
            const result = await api.uploadFile(file)
            setProgress((p) => ({ ...p, [file.name]: 100 }))
            return result as UploadedFile
          })
        )
        onFilesChange([...files, ...uploaded])
      } catch (err) {
        console.error('Upload error:', err)
      } finally {
        setUploading(false)
        setProgress({})
      }
    },
    [files, maxFiles, onFilesChange]
  )

  const removeFile = (index: number) => {
    onFilesChange(files.filter((_, i) => i !== index))
  }

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    accept: {
      'image/*': ['.png', '.jpg', '.jpeg', '.gif', '.webp'],
      'application/pdf': ['.pdf'],
    },
    maxSize: 10 * 1024 * 1024,
    maxFiles: maxFiles - files.length,
    disabled: uploading,
  })

  return (
    <div className="space-y-4">
      <div
        {...getRootProps()}
        className={`border-2 border-dashed rounded-2xl p-8 text-center cursor-pointer transition-colors
          ${isDragActive ? 'border-accent bg-accent/5' : 'border-gray-300 hover:border-accent/50'}
          ${uploading ? 'opacity-50 cursor-not-allowed' : ''}`}
      >
        <input {...getInputProps()} />
        <div className="text-gray-400">
          <svg className="w-10 h-10 mx-auto mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
          </svg>
          <p className="text-sm">
            {isDragActive ? 'Drop files here...' : 'Drag & drop files, or click to browse'}
          </p>
          <p className="text-xs text-gray-400 mt-1">
            Images & PDFs, up to 10MB each (max {maxFiles} files)
          </p>
        </div>
      </div>

      {Object.entries(progress).map(([name, pct]) => (
        <div key={name} className="flex items-center gap-3">
          <span className="text-sm text-gray-500 truncate flex-1">{name}</span>
          <div className="w-24 h-2 bg-gray-200 rounded-full overflow-hidden">
            <div className="h-full bg-accent rounded-full transition-all" style={{ width: `${pct}%` }} />
          </div>
        </div>
      ))}

      {files.length > 0 && (
        <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
          {files.map((file, i) => (
            <div key={file.storage_key} className="relative group rounded-xl overflow-hidden border border-gray-200">
              {file.file_type.startsWith('image/') ? (
                <img src={file.url} alt={file.filename} className="w-full h-24 object-cover" />
              ) : (
                <div className="w-full h-24 bg-gray-50 flex items-center justify-center text-gray-400 text-xs">
                  PDF
                </div>
              )}
              <button
                type="button"
                onClick={() => removeFile(i)}
                className="absolute top-1 right-1 w-6 h-6 bg-red-500 text-white rounded-full text-xs
                  opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
              >
                &times;
              </button>
              <p className="text-xs text-gray-500 p-1 truncate">{file.filename}</p>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
