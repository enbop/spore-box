import React, { useState, useEffect, useRef } from 'react';
import MessageItem from './components/MessageItem';
import MessageInput from './components/MessageInput';
import { Message } from './types';
import { api } from './api';

const Chat: React.FC = () => {
    const [messages, setMessages] = useState<Message[]>([]);
    const [loading, setLoading] = useState(false);
    const [deviceName, setDeviceName] = useState('Browser');
    const messagesEndRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const initDevice = async () => {
            const name = await api.getDeviceName();
            console.log('Device name from URL:', name); // Add debug log
            setDeviceName(name);
        };
        initDevice();
    }, []);

    useEffect(() => {
        const loadMessages = async () => {
            try {
                setLoading(true);
                const msgs = await api.getMessages();
                setMessages(msgs);
            } catch (error) {
                console.error('Failed to load messages:', error);
            } finally {
                setLoading(false);
            }
        };
        loadMessages();
    }, []);

    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages]);

    const handleSendMessage = async (content: string, type: 'text' | 'image' | 'file', file?: File) => {
        try {
            setLoading(true);
            let newMessage: Message;

            if (type === 'text') {
                newMessage = await api.sendMessage({
                    content,
                    sender: deviceName,
                    type
                });
            } else if (file) {
                newMessage = await api.uploadFile(file, deviceName);
            } else {
                return;
            }

            setMessages(prev => [...prev, newMessage]);
        } catch (error) {
            console.error('Failed to send message:', error);
            alert('Failed to send message. Please try again.');
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className="fixed inset-0 flex flex-col bg-gray-50">
            <header className="bg-white border-b border-gray-200 p-4 flex-shrink-0">
                <h1 className="text-xl font-semibold text-gray-800">Spore Box</h1>
                <p className="text-sm text-gray-500">Device: {deviceName}</p>
            </header>

            <div className="flex-1 overflow-y-auto p-4" style={{ paddingBottom: '80px' }}>
                {loading && messages.length === 0 ? (
                    <div className="flex items-center justify-center h-full">
                        <div className="text-gray-500">Loading messages...</div>
                    </div>
                ) : messages.length === 0 ? (
                    <div className="flex items-center justify-center h-full">
                        <div className="text-center">
                            <div className="text-gray-500 mb-2">No messages yet</div>
                            <div className="text-sm text-gray-400">Send your first message to get started!</div>
                        </div>
                    </div>
                ) : (
                    messages.map((message) => (
                        <MessageItem
                            key={message.id}
                            message={message}
                            isOwn={message.sender === deviceName}
                        />
                    ))
                )}
                <div ref={messagesEndRef} />
            </div>

            <div className="fixed bottom-0 left-0 right-0 bg-white border-t border-gray-200">
                <MessageInput onSendMessage={handleSendMessage} disabled={loading} />
            </div>
        </div>
    );
};

export default Chat;
