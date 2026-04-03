// React Media Upload Component
// nexus-web/src/components/MediaUpload.tsx

import React, { useState, useRef } from 'react';
import { Upload, X } from 'lucide-react';

interface UploadProgress {
  fileId: string;
  fileName: string;
  fileSize: number; // Track actual file size
  progress: number;
  status: 'uploading' | 'complete' | 'error';
  error?: string;
}

const MediaUpload: React.FC = () => {
  const [uploads, setUploads] = useState<UploadProgress[]>([]);
  const [isDragActive, setIsDragActive] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === 'dragenter' || e.type === 'dragover') {
      setIsDragActive(true);
    } else if (e.type === 'dragleave') {
      setIsDragActive(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragActive(false);

    const files = e.dataTransfer.files;
    if (files) {
      handleFiles(files);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files) {
      handleFiles(e.target.files);
    }
  };

  const handleFiles = (files: FileList) => {
    Array.from(files).forEach((file) => {
      uploadFile(file);
    });
  };

  const uploadFile = (file: File) => {
    const MAX_FILE_SIZE = 1024 * 1024 * 1024; // 1GB in bytes
    const fileSize = file.size;

    // Validate file size client-side
    if (fileSize > MAX_FILE_SIZE) {
      const errorMsg = `File too large: ${formatFileSize(fileSize)} exceeds 1GB limit`;
      setUploads((prev) => [
        ...prev,
        {
          fileId: `${Date.now()}_${Math.random()}`,
          fileName: file.name,
          fileSize,
          progress: 0,
          status: 'error' as const,
          error: errorMsg,
        },
      ]);
      return;
    }

    const fileId = `${Date.now()}_${Math.random()}`;
    const chunkSize = 1024 * 1024; // 1MB chunks

    // Add to uploads list
    setUploads((prev) => [
      ...prev,
      {
        fileId,
        fileName: file.name,
        fileSize,
        progress: 0,
        status: 'uploading' as const,
      },
    ]);

    let uploadedBytes = 0;
    let chunkIndex = 0;

    const uploadNextChunk = () => {
      if (uploadedBytes >= fileSize) {
        // Upload complete
        setUploads((prev) =>
          prev.map((u) =>
            u.fileId === fileId ? { ...u, status: 'complete' as const, progress: 100 } : u
          )
        );
        return;
      }

      const chunk = file.slice(uploadedBytes, uploadedBytes + chunkSize);
      const formData = new FormData();
      formData.append('file_id', fileId);
      formData.append('chunk_index', chunkIndex.toString());
      formData.append('total_chunks', Math.ceil(fileSize / chunkSize).toString());
      formData.append('chunk', chunk);

      fetch('/api/upload/chunk', {
        method: 'POST',
        body: formData,
      })
        .then(() => {
          uploadedBytes += chunk.size;
          chunkIndex++;
          const progress = Math.round((uploadedBytes / fileSize) * 100);

          setUploads((prev) =>
            prev.map((u) =>
              u.fileId === fileId ? { ...u, progress } : u
            )
          );

          uploadNextChunk();
        })
        .catch((error) => {
          setUploads((prev) =>
            prev.map((u) =>
              u.fileId === fileId
                ? { ...u, status: 'error' as const, error: error.message }
                : u
            )
          );
        });
    };

    uploadNextChunk();
  };

  const removeUpload = (fileId: string) => {
    setUploads((prev) => prev.filter((u) => u.fileId !== fileId));
  };

  const retryUpload = (fileId: string) => {
    const upload = uploads.find((u) => u.fileId === fileId);
    if (upload) {
      removeUpload(fileId);
      // Retry logic would go here
    }
  };

  return (
    <div className="w-full max-w-2xl mx-auto p-6">
      {/* Upload Area */}
      <div
        onDragEnter={handleDrag}
        onDragLeave={handleDrag}
        onDragOver={handleDrag}
        onDrop={handleDrop}
        className={`border-2 border-dashed rounded-lg p-8 text-center transition ${
          isDragActive
            ? 'border-blue-500 bg-blue-50'
            : 'border-gray-300 bg-gray-50 hover:bg-gray-100'
        }`}
      >
        <input
          ref={fileInputRef}
          type="file"
          multiple
          onChange={handleChange}
          className="hidden"
          accept="image/*,video/*,audio/*,.pdf,.doc,.docx,.txt"
        />

        <Upload size={32} className="mx-auto mb-4 text-gray-400" />

        <p className="text-lg font-semibold text-gray-700 mb-2">
          Drag files here or click to browse
        </p>
        <p className="text-sm text-gray-500 mb-4">
          Supported: Images, Video, Audio, Documents (Max 1GB per file)
        </p>

        <button
          onClick={() => fileInputRef.current?.click()}
          className="bg-blue-500 hover:bg-blue-600 text-white font-semibold py-2 px-6 rounded-lg transition"
        >
          Select Files
        </button>
      </div>

      {/* Upload Progress */}
      {uploads.length > 0 && (
        <div className="mt-8">
          <h3 className="text-lg font-semibold mb-4">Uploads ({uploads.length})</h3>

          <div className="space-y-4">
            {uploads.map((upload) => (
              <div
                key={upload.fileId}
                className="bg-white rounded-lg p-4 border border-gray-200"
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="font-medium text-gray-800 truncate">
                    {upload.fileName}
                  </span>

                  {upload.status === 'complete' && (
                    <span className="text-green-600 text-sm font-medium">✓ Complete</span>
                  )}
                  {upload.status === 'error' && (
                    <div className="flex items-center gap-2">
                      <span className="text-red-600 text-sm font-medium">{upload.error}</span>
                      <button
                        onClick={() => retryUpload(upload.fileId)}
                        className="text-blue-600 hover:text-blue-800 text-sm font-medium"
                      >
                        Retry
                      </button>
                    </div>
                  )}

                  <button
                    onClick={() => removeUpload(upload.fileId)}
                    className="text-gray-400 hover:text-gray-600"
                  >
                    <X size={20} />
                  </button>
                </div>

                {/* Progress Bar */}
                <div className="w-full bg-gray-200 rounded-full h-2 overflow-hidden">
                  <div
                    className={`h-full transition-all duration-300 ${
                      upload.status === 'error'
                        ? 'bg-red-500'
                        : upload.status === 'complete'
                        ? 'bg-green-500'
                        : 'bg-blue-500'
                    }`}
                    style={{ width: `${upload.progress}%` }}
                  />
                </div>

                <div className="mt-2 text-sm text-gray-600">
                  {upload.progress}% • {formatFileSize(upload.fileSize)}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
};

export default MediaUpload;
