import React from "react";
import "./ServerCard.css";

interface ServerCardProps {
  name: string;
  author: string;
  downloads: number;
  description: string;
  repoUrl: string;
}

const ServerCard: React.FC<ServerCardProps> = ({
  name,
  author,
  downloads,
  description,
}) => {
  return (
    <div className="server-card">
      <div className="server-card-header">
        <div className="server-info">
          <h3>{name}</h3>
          <span className="author">@{author}</span>
        </div>
        <div className="downloads">
          <span>{downloads.toLocaleString()}</span>
        </div>
      </div>
      <p className="description">{description}</p>
    </div>
  );
};

export default ServerCard;
