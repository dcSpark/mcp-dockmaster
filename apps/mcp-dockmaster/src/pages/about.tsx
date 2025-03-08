const About = () => {
  return (
    <div className="text-muted-foreground mx-auto flex h-full w-full max-w-4xl flex-col gap-8 px-6 py-10 text-sm">
      <h1 className="text-foreground text-2xl font-semibold tracking-tight">
        About MCP Dockmaster
      </h1>
      <section className="about-section">
        <h2 className="text-foreground mb-4 border-b border-gray-300 pb-2 text-lg font-semibold">
          Overview
        </h2>
        <p>
          MCP Dockmaster is a powerful application management platform designed
          to simplify the deployment, management, and monitoring of AI
          applications and services. Our platform provides a seamless experience
          for developers and users alike.
        </p>
      </section>
      <section className="about-section">
        <h2 className="text-foreground mb-4 border-b border-gray-300 pb-2 text-lg font-semibold">
          Mission
        </h2>
        <p>
          Our mission is to democratize access to advanced AI tools and
          applications by providing an intuitive interface for managing complex
          software ecosystems. We believe in making technology accessible to
          everyone, regardless of their technical expertise.
        </p>
      </section>
      <section className="about-section">
        <h2 className="text-foreground mb-4 border-b border-gray-300 pb-2 text-lg font-semibold">
          Features
        </h2>
        <ul className="ml-6 list-disc">
          <li className="mb-2">Easy application installation and management</li>
          <li className="mb-2">
            Centralized control panel for all your AI applications
          </li>
          <li className="mb-2">Access to a curated store of AI applications</li>
          <li className="mb-2">Simplified configuration and deployment</li>
          <li className="mb-2">Real-time monitoring and logging</li>
        </ul>
      </section>
      <section className="about-section">
        <h2 className="text-foreground mb-4 border-b border-gray-300 pb-2 text-lg font-semibold">
          Contact
        </h2>
        <p>
          For more information about MCP Dockmaster, please visit our website or
          contact our support team.
        </p>
      </section>
    </div>
  );
};

export default About;
