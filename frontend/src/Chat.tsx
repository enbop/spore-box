import React, { useState, useEffect, useRef } from 'react';
import MessageItem from './components/MessageItem';
import MessageInput from './components/MessageInput';
import { Message } from './types';
import { api } from './api';

// Polling configuration constants
const POLLING_CONFIG = {
    INITIAL_INTERVAL: 3000,      // 3 seconds initial polling interval
    MAX_INTERVAL: 30000,         // 30 seconds maximum polling interval
    SUCCESS_MULTIPLIER: 1,       // Reset to initial on success
    IDLE_MULTIPLIER: 1.5,        // Increase by 1.5x when no new messages
    ERROR_MULTIPLIER: 2,         // Double interval on error
};

const Chat: React.FC = () => {
    const [messages, setMessages] = useState<Message[]>([]);
    const [loading, setLoading] = useState(false);
    const [deviceName, setDeviceName] = useState('Browser');
    const [shouldAutoScroll, setShouldAutoScroll] = useState(true);
    const [imagesLoaded, setImagesLoaded] = useState(0);
    const messagesEndRef = useRef<HTMLDivElement>(null);
    const messagesContainerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const initDevice = async () => {
            const name = await api.getDeviceName();
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
        let pollInterval: NodeJS.Timeout | null = null;
        let lastTimestamp = new Date().toISOString();
        let isActive = true;
        let currentPollInterval = POLLING_CONFIG.INITIAL_INTERVAL;

        const poll = async () => {
            if (!isActive) return;

            try {
                const response = await api.pollMessages(lastTimestamp);
                if (response.messages && response.messages.length > 0) {
                    setMessages(prev => {
                        const existingIds = new Set(prev.map(m => m.id));
                        const newMessages = response.messages.filter(m => !existingIds.has(m.id));
                        
                        // Update messages only if there are new ones
                        if (newMessages.length > 0) {
                            return [...prev, ...newMessages];
                        }
                        return prev; // No new messages, return previous state
                    });
                    
                    currentPollInterval = POLLING_CONFIG.INITIAL_INTERVAL;
                } else {
                    currentPollInterval = Math.min(
                        currentPollInterval * POLLING_CONFIG.IDLE_MULTIPLIER, 
                        POLLING_CONFIG.MAX_INTERVAL
                    );
                }
                
                lastTimestamp = response.timestamp;
            } catch (error) {
                console.error('Polling failed:', error);
                currentPollInterval = Math.min(
                    currentPollInterval * POLLING_CONFIG.ERROR_MULTIPLIER, 
                    POLLING_CONFIG.MAX_INTERVAL
                );
            }

            if (isActive) {
                pollInterval = setTimeout(poll, currentPollInterval);
            }
        };

        const handleVisibilityChange = () => {
            if (document.hidden) {
                isActive = false;
                if (pollInterval) {
                    clearTimeout(pollInterval);
                    pollInterval = null;
                }
            } else {
                if (!isActive) {
                    isActive = true;
                    currentPollInterval = POLLING_CONFIG.INITIAL_INTERVAL;
                    poll();
                }
            }
        };

        document.addEventListener('visibilitychange', handleVisibilityChange);

        poll();

        return () => {
            isActive = false;
            if (pollInterval) {
                clearTimeout(pollInterval);
            }
            document.removeEventListener('visibilitychange', handleVisibilityChange);
        };
    }, []);

    useEffect(() => {
        const checkIfAtBottom = () => {
            if (messagesContainerRef.current) {
                const { scrollTop, scrollHeight, clientHeight } = messagesContainerRef.current;
                const isAtBottom = scrollHeight - scrollTop - clientHeight < 100; // 100px 容差
                setShouldAutoScroll(isAtBottom);
            }
        };

        if (shouldAutoScroll) {
            messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
        }

        const container = messagesContainerRef.current;
        if (container) {
            container.addEventListener('scroll', checkIfAtBottom);
            return () => container.removeEventListener('scroll', checkIfAtBottom);
        }
    }, [messages, shouldAutoScroll, imagesLoaded]); // 添加 imagesLoaded 依赖

    const handleImageLoad = () => {
        setImagesLoaded(prev => prev + 1);
        if (shouldAutoScroll) {
            setTimeout(() => {
                messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
            }, 100);
        }
    };

    useEffect(() => {
        if (messages.length > 0 && !loading) {
            const timer = setTimeout(() => {
                if (shouldAutoScroll) {
                    messagesEndRef.current?.scrollIntoView({ behavior: 'auto' });
                }
            }, 300);
            
            return () => clearTimeout(timer);
        }
    }, [loading, messages.length, shouldAutoScroll]);

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
            setShouldAutoScroll(true);
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

            <div ref={messagesContainerRef} className="flex-1 overflow-y-auto p-4" style={{ paddingBottom: '80px' }}>
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
                            onImageLoad={handleImageLoad}
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
