export interface Message {
  id: string;
  content: string;
  sender: string;
  timestamp: string;
  type: 'text' | 'image' | 'file';
  filename?: string;
  fileSize?: number;
  mimeType?: string;
}

export interface SendMessageRequest {
  content: string;
  sender: string;
  type: 'text' | 'image' | 'file';
  filename?: string;
}
