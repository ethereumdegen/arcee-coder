import type { UploadedFile } from '../../types'
import FileDropzone from '../ui/FileDropzone'

interface StepUploadProps {
  files: UploadedFile[]
  onFilesChange: (files: UploadedFile[]) => void
  uploadsEnabled: boolean
}

export default function StepUpload({ files, onFilesChange, uploadsEnabled }: StepUploadProps) {
  return (
    <div>
      <h3 className="text-2xl font-heading font-bold text-gray-900 mb-2">Upload Files</h3>
      <p className="text-gray-500 mb-8">
        Share any relevant files — mockups, briefs, brand assets. This step is optional.
      </p>
      {uploadsEnabled ? (
        <FileDropzone files={files} onFilesChange={onFilesChange} />
      ) : (
        <div className="border-2 border-dashed border-gray-200 rounded-2xl p-8 text-center text-gray-400 text-sm">
          File uploads are not available right now. You can skip this step.
        </div>
      )}
    </div>
  )
}
