import { Outlet, useOutletContext } from 'react-router-dom';
import SubNav, { type SubNavItem } from '@/components/SubNav';

const tabs: SubNavItem[] = [
  { to: 'categories', label: 'Categories' },
  { to: 'applicability', label: 'Applicability' },
  { to: 'requirement-statuses', label: 'Requirement statuses' },
  { to: 'verification-statuses', label: 'Verification statuses' },
  { to: 'custom-fields', label: 'Custom fields' },
  { to: 'verification-methods', label: 'Verification methods' },
];

/** Project settings › Catalog: second-level tabs over the catalog pages. */
export default function ProjectCatalogLayout() {
  const context = useOutletContext();
  return (
    <div>
      <p className="text-sm text-stitch-muted mb-4">
        Categories, applicability, statuses, custom fields and verification methods used by this project.
      </p>
      <SubNav items={tabs} ariaLabel="Catalog sections" variant="secondary" />
      <Outlet context={context} />
    </div>
  );
}
