import { NextResponse } from "next/server";
import { pipe as browserPipe } from "@screenpipe/browser";

export async function GET(request: Request) {
  try {
    // Get query parameters
    const url = new URL(request.url);
    const app = url.searchParams.get("app") || "Chrome";
    const role = url.searchParams.get("role") || "AXButton";
    const maxResults = parseInt(url.searchParams.get("maxResults") || "10");
    const maxDepth = parseInt(url.searchParams.get("maxDepth") || "1");
    
    console.log(`searching for ${role} elements in ${app} app`);
    
    // Use the Operator SDK to locate UI elements
    const elements = await browserPipe.operator
      .locator({
        app: app,
        role: role,
        useBackgroundApps: true,
        activateApp: false, // Default to not activating to avoid disrupting the user
      })
      .all(maxResults, maxDepth);
    
    console.log(`found ${elements.length} elements`);
    
    return NextResponse.json({
      app,
      role,
      elementsFound: elements.length,
      elements,
    });
  } catch (error) {
    console.error("error accessing ui elements:", error);
    return NextResponse.json(
      { error: `failed to access ui elements: ${error}` },
      { status: 500 }
    );
  }
}