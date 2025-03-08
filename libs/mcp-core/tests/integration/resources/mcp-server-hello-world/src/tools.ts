import { helloWorldHandler, helloWorldWithInputHandler, helloWorldWithConfigHandler } from "./handlers/hello-world";

export const tools = [
  {
    name: "hello_world",
    description: "Returns hello world",
    inputSchema: {
      type: "object",
      properties: {},
      required: []
    }
  },
  {
    name: "hello_world_with_input",
    description: "Returns hello world with the provided input",
    inputSchema: {
      type: "object",
      properties: {
        message: { type: "string" }
      },
      required: []
    }
  },
  {
    name: "hello_world_with_config",
    description: "Returns hello configuration with the provided config",
    inputSchema: {
      type: "object",
      properties: {
        config: { type: "string" }
      },
      required: ["config"]
    }
  }
];

type handlerDictionary = Record<typeof tools[number]["name"], (input: any) => any>;

export const handlers: handlerDictionary = {
  "hello_world": helloWorldHandler,
  "hello_world_with_input": helloWorldWithInputHandler,
  "hello_world_with_config": helloWorldWithConfigHandler
};
