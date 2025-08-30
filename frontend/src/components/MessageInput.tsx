import React, { useState, useRef } from 'react';
import { Send, Paperclip, ImageIcon } from 'lucide-react';

interface MessageInputProps {
    onSendMessage: (content: string, type: 'text' | 'image' | 'file', file?: File) => void;
    disabled?: boolean;
}

const MessageInput: React.FC<MessageInputProps> = ({ onSendMessage, disabled }) => {
    const [message, setMessage] = useState('');
    const fileInputRef = useRef<HTMLInputElement>(null);
    const imageInputRef = useRef<HTMLInputElement>(null);
    const textInputRef = useRef<HTMLInputElement>(null);

    // Auto focus on mount
    React.useEffect(() => {
        if (textInputRef.current && !disabled) {
            textInputRef.current.focus();
        }
    }, [disabled]);

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        if (message.trim() && !disabled) {
            const content = message.trim();
            setMessage(''); // Clear message first

            // Send the message
            onSendMessage(content, 'text');

            // Refocus the input after a short delay to ensure it works
            requestAnimationFrame(() => {
                if (textInputRef.current) {
                    textInputRef.current.focus();
                }
            });
        }
    }; const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>, type: 'image' | 'file') => {
        const file = e.target.files?.[0];
        if (file) {
            onSendMessage('', type, file);
            e.target.value = '';

            // Refocus the text input after file upload
            requestAnimationFrame(() => {
                if (textInputRef.current) {
                    textInputRef.current.focus();
                }
            });
        }
    };

    const handlePaste = async (e: React.ClipboardEvent) => {
        const items = e.clipboardData.items;
        for (let i = 0; i < items.length; i++) {
            const item = items[i];
            if (item.type.indexOf('image') !== -1) {
                const file = item.getAsFile();
                if (file) {
                    onSendMessage('', 'image', file);

                    // Refocus the text input after paste
                    requestAnimationFrame(() => {
                        if (textInputRef.current) {
                            textInputRef.current.focus();
                        }
                    });
                    return;
                }
            }
        }
    };

    return (
        <div className="p-4">
            <form onSubmit={handleSubmit} className="flex items-center gap-2">
                <input
                    type="file"
                    ref={fileInputRef}
                    onChange={(e) => handleFileSelect(e, 'file')}
                    className="hidden"
                />
                <input
                    type="file"
                    ref={imageInputRef}
                    onChange={(e) => handleFileSelect(e, 'image')}
                    accept="image/*"
                    className="hidden"
                />

                <button
                    type="button"
                    onClick={() => fileInputRef.current?.click()}
                    disabled={disabled}
                    className="flex-shrink-0 p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-full disabled:opacity-50 touch-manipulation"
                >
                    <Paperclip size={20} />
                </button>

                <button
                    type="button"
                    onClick={() => imageInputRef.current?.click()}
                    disabled={disabled}
                    className="flex-shrink-0 p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-full disabled:opacity-50 touch-manipulation"
                >
                    <ImageIcon size={20} />
                </button>

                <input
                    ref={textInputRef}
                    type="text"
                    value={message}
                    onChange={(e) => setMessage(e.target.value)}
                    onPaste={handlePaste}
                    placeholder="Type a message..."
                    disabled={disabled}
                    className="flex-1 min-w-0 border border-gray-300 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50"
                    style={{ fontSize: '16px' }} // Prevent zoom on iOS
                />

                <button
                    type="submit"
                    disabled={disabled || !message.trim()}
                    className="flex-shrink-0 p-2 bg-blue-500 text-white rounded-full hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed touch-manipulation"
                >
                    <Send size={20} />
                </button>
            </form>
        </div>
    );
};

export default MessageInput;
