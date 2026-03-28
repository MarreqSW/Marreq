export type ProjectOutletContext = {
  projectId: number;
  basePath: string;
  globalSearch: string;
  setGlobalSearch: (q: string) => void;
};
