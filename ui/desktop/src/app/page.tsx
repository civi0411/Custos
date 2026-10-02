import React from 'react';
import { Navigate } from 'react-router-dom';

/**
 * Home page (Route `/`)
 * App Router entrypoint redirecting to `/studio`
 */
export const Page: React.FC = () => {
  return <Navigate to="/studio" replace />;
};

export default Page;
