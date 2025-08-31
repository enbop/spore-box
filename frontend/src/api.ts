import axios from "axios";
import { Message, SendMessageRequest } from "./types";

const API_BASE = "/api";

export const api = {
  async getMessages(): Promise<Message[]> {
    const response = await axios.get(`${API_BASE}/messages`);
    return response.data;
  },

  async sendMessage(data: SendMessageRequest): Promise<Message> {
    const response = await axios.post(`${API_BASE}/messages`, data);
    return response.data;
  },

  async uploadFile(file: File, sender: string): Promise<Message> {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("sender", sender);

    const response = await axios.post(`${API_BASE}/upload`, formData, {
      headers: {
        "Content-Type": "multipart/form-data",
      },
    });
    return response.data;
  },

  async pollMessages(
    since: string
  ): Promise<{ messages: Message[]; timestamp: string }> {
    const response = await axios.get(
      `${API_BASE}/messages/poll?since=${encodeURIComponent(since)}`
    );
    return response.data;
  },

  async getDeviceName(): Promise<string> {
    const params = new URLSearchParams(window.location.search);
    return params.get("device") || "Browser";
  },
};
