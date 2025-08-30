import React from 'react';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { dark } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { Message } from '../types';
import { Download, FileText, Image } from 'lucide-react';

interface MessageItemProps {
    message: Message;
    isOwn: boolean;
}

const MessageItem: React.FC<MessageItemProps> = ({ message, isOwn }) => {
    const formatTime = (timestamp: string) => {
        return new Date(timestamp).toLocaleTimeString([], {
            hour: '2-digit',
            minute: '2-digit'
        });
    };

    const formatFileSize = (bytes?: number) => {
        if (!bytes) return '';
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(1024));
        return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
    };

    const renderContent = () => {
        switch (message.type) {
            case 'image':
                return (
                    <div className="space-y-2">
                        <img
                            src={`/api/files/${message.content}`}
                            alt={message.filename}
                            className="max-w-sm rounded-lg cursor-pointer"
                            onClick={() => window.open(`/api/files/${message.content}`, '_blank')}
                        />
                        <div className="text-xs text-gray-500 flex items-center gap-1">
                            <Image size={12} />
                            {message.filename} ({formatFileSize(message.fileSize)})
                        </div>
                    </div>
                );

            case 'file':
                return (
                    <div className="border border-gray-300 rounded-lg p-3 max-w-xs">
                        <div className="flex items-center gap-2 mb-1">
                            <FileText size={16} />
                            <span className="text-sm font-medium truncate">{message.filename}</span>
                        </div>
                        <div className="text-xs text-gray-500 mb-2">
                            {formatFileSize(message.fileSize)}
                        </div>
                        <a
                            href={`/api/files/${message.content}`}
                            download={message.filename}
                            className="inline-flex items-center gap-1 text-blue-600 hover:text-blue-800 text-sm"
                        >
                            <Download size={12} />
                            Download
                        </a>
                    </div>
                );

            default:
                return (
                    <div className="prose prose-sm max-w-none">
                        <ReactMarkdown
                            components={{
                                code({ className, children, ...props }: any) {
                                    const match = /language-(\w+)/.exec(className || '');
                                    const isInline = !match;
                                    return !isInline ? (
                                        <SyntaxHighlighter
                                            style={dark as any}
                                            language={match[1]}
                                            PreTag="div"
                                        >
                                            {String(children).replace(/\n$/, '')}
                                        </SyntaxHighlighter>
                                    ) : (
                                        <code className={className} {...props}>
                                            {children}
                                        </code>
                                    );
                                }
                            }}
                        >
                            {message.content}
                        </ReactMarkdown>
                    </div>
                );
        }
    };

    return (
        <div className={`flex ${isOwn ? 'justify-end' : 'justify-start'} mb-4`}>
            <div className={`max-w-[85%] sm:max-w-[70%] ${isOwn ? 'order-1' : 'order-2'}`}>
                <div
                    className={`rounded-lg px-4 py-2 break-words overflow-hidden ${isOwn
                        ? 'bg-blue-500 text-white'
                        : 'bg-gray-100 text-gray-900'
                        }`}
                >
                    {renderContent()}
                </div>
                <div
                    className={`text-xs text-gray-500 mt-1 ${isOwn ? 'text-right' : 'text-left'
                        }`}
                >
                    {message.sender} · {formatTime(message.timestamp)}
                </div>
            </div>
        </div>
    );
};

export default MessageItem;
