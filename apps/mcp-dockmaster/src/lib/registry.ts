import { Registry, RegistryServer } from "./mcpClient";

interface CacheEntry {
  timestamp: number;
  data: RegistryServer[];
  categories: [string, number][];
}

let serversCache: CacheEntry | null = null;
const CACHE_DURATION = 60 * 1000;  // 60 seconds
//  24 * 60 * 60 * 1000; // 24 hours in milliseconds

/**
* Get all available tools from the registry
* @param force If true, bypasses cache and forces a new download
*/
export const getAvailableServers = async (force: boolean = false): Promise<RegistryServer[]> => {
  // Return cached data if available and not expired and force is false
  const cacheValid = serversCache && (Date.now() - serversCache.timestamp) < CACHE_DURATION;
  if (!force && cacheValid) {
    return serversCache!.data;
  }

  try {
    // All Tools: Stable & Unstable
    const response = await fetch('https://pub-5e2d77d67aac45ef811998185d312005.r2.dev/registry/registry.all.json');
    // Stable Only
    // const response = await fetch('https://pub-5e2d77d67aac45ef811998185d312005.r2.dev/registry/registry.stable.json');
    // Unstable Only
    // const response = await fetch('https://pub-5e2d77d67aac45ef811998185d312005.r2.dev/registry/registry.unstable.json');
    if (!response.ok) {
      throw new Error(`Failed to fetch tools: ${response.statusText}`);
    }
    const servers: Registry = await response.json();
    
    const categoryCounts: Record<string, number> = {};
    servers.tools.forEach((server) => {
      server.categories?.forEach((category) => {
        categoryCounts[category] = (categoryCounts[category] || 0) + 1;
      });
    });
    const uniqueCategoriesOrdered = Object.keys(categoryCounts)
      .sort((a, b) => categoryCounts[b] - categoryCounts[a])
      .map((category) => [category, categoryCounts[category]] as [string, number]);
    
    // Update cache
    serversCache = {
      timestamp: Date.now(),
      data: servers.tools,
      categories: uniqueCategoriesOrdered
    };
    
    return servers.tools;
  } catch (error) {
    console.error('Error fetching available tools:', error);
    // If cache exists, return cached data even if expired
    if (serversCache?.data) {
      console.warn('Returning expired cached data due to fetch error');
      return serversCache.data;
    }
    return [];
  }
};

const wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

export const getCategories = async (): Promise<[string, number][]> => {
  // Wait for serversCache to be populated, but with a timeout
  let attempts = 0;
  const maxAttempts = 100; // 5 seconds max wait (100 * 50ms)
  
  while (!serversCache && attempts < maxAttempts) {
    await wait(50);
    attempts++;
  }
  
  // Return empty array if cache is still not available after timeout
  if (!serversCache) {
    console.warn('getCategories timeout: serversCache not available');
    return [];
  }
  
  return serversCache.categories;
};

export const getCategoryServers = async (category: string): Promise<RegistryServer[]> => {
  while (!serversCache) await wait(50);
  return serversCache.data.filter((server: RegistryServer) => server.categories?.includes(category));  
};

/**
* Get a specific tool by ID
*/
export const getServerById = async (id: string): Promise<RegistryServer | null> => {
  if (serversCache) {
    return serversCache.data.find((server: RegistryServer) => server.id === id) || null;
  }
  return null;
};

export default {
  getAvailableServers,
  getServerById
};
