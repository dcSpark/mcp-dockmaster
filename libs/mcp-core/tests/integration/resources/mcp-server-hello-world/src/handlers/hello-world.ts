import { ToolResultSchema } from "../types";
import { createErrorResponse, createSuccessResponse } from "./utils";
import { HelloWorldInput, HelloWorldConfigInput } from "./hello-world.types";

export const helloWorldHandler = async (): Promise<ToolResultSchema<any>> => {
  try {
    return createSuccessResponse("hello world");
  } catch (error) {
    return createErrorResponse(`Error in hello_world: ${error instanceof Error ? error.message : String(error)}`);
  }
};

export const helloWorldWithInputHandler = async (input: HelloWorldInput): Promise<ToolResultSchema<any>> => {
  try {
    const message = input.message || "";
    return createSuccessResponse(`hello world ${message}`);
  } catch (error) {
    return createErrorResponse(`Error in hello_world_with_input: ${error instanceof Error ? error.message : String(error)}`);
  }
};

export const helloWorldWithConfigHandler = async (input: HelloWorldConfigInput): Promise<ToolResultSchema<any>> => {
  try {
    if (!input.config) {
      return createErrorResponse("Config is required");
    }
    return createSuccessResponse(`hello configuration ${input.config}`);
  } catch (error) {
    return createErrorResponse(`Error in hello_world_with_config: ${error instanceof Error ? error.message : String(error)}`);
  }
};
