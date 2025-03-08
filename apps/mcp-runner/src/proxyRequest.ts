import { debugLog } from "./logger";
// Target server URL

const TARGET_SERVER_URL = 'http://localhost:3000/mcp-proxy';
console.error(`Target server: ${TARGET_SERVER_URL}`);

/**
 * Generic proxy function to forward requests to the target server
 * @param method The JSON-RPC method to call
 * @param params The parameters to pass to the method
 * @returns The response from the target server
 */
export async function proxyRequest<T>(method: string, params: any): Promise<T> {
    debugLog(`proxyRequest called with method: ${method}`);
    try {
      console.error(`Proxying request: ${method} to ${TARGET_SERVER_URL}`);
      console.error(`Request params: ${JSON.stringify(params)}`);
      
      // The server only accepts POST requests with JSON-RPC format
      const requestBody = JSON.stringify({
        jsonrpc: '2.0',
        id: 1,
        method,
        params: params || {},
      });
      
      console.error(`Request body: ${requestBody}`);
      
      const response = await fetch(TARGET_SERVER_URL, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: requestBody,
      });
  
      if (!response.ok) {
        const responseText = await response.text();
        console.error(`HTTP error! status: ${response.status}, response: ${responseText}`);
        throw new Error(`HTTP error! status: ${response.status}, response: ${responseText}`);
      }
  
      const data = await response.json() as { result: any; error?: { message: string } };
      
      if (data.error) {
        console.error(`Server error: ${JSON.stringify(data.error)}`);
        throw new Error(`Server error: ${JSON.stringify(data.error)}`);
      }
  
      console.error(`Received response for: ${method}`);
      console.error(`Response data: ${JSON.stringify(data.result).substring(0, 100)}...`);
      return data.result;
    } catch (error) {
      console.error(`Error proxying request: ${error}`);
      throw error;
    }
  }