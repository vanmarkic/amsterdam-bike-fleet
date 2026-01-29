import { Component, OnInit, OnDestroy, ChangeDetectionStrategy, ChangeDetectorRef } from '@angular/core';

import { Subject } from 'rxjs';
import { takeUntil, finalize } from 'rxjs/operators';
import { Issue, IssueCategory, IssueReporterType } from '../../models/fleet.models';
import { IssueService, IssueFilters } from '../../services/issue.service';

type ResolvedFilter = 'all' | 'resolved' | 'unresolved';
type CategoryFilter = 'all' | IssueCategory;

interface CategoryInfo {
  label: string;
  icon: string;
}

@Component({
  selector: 'app-issues-page',
  standalone: true,
  imports: [],
  templateUrl: './issues-page.component.html',
  styleUrls: ['./issues-page.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class IssuesPageComponent implements OnInit, OnDestroy {
  issues: Issue[] = [];
  selectedIssue: Issue | null = null;
  loading = false;
  error: string | null = null;

  resolvedFilter: ResolvedFilter = 'all';
  categoryFilter: CategoryFilter = 'all';

  readonly resolvedOptions: { value: ResolvedFilter; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'resolved', label: 'Resolved' },
    { value: 'unresolved', label: 'Unresolved' }
  ];

  readonly categoryOptions: { value: CategoryFilter; label: string }[] = [
    { value: 'all', label: 'All Categories' },
    { value: 'late', label: 'Late' },
    { value: 'damaged', label: 'Damaged' },
    { value: 'wrong_order', label: 'Wrong Order' },
    { value: 'rude', label: 'Rude' },
    { value: 'bike_problem', label: 'Bike Problem' },
    { value: 'other', label: 'Other' }
  ];

  readonly categoryInfoMap: Record<IssueCategory, CategoryInfo> = {
    late: { label: 'Late', icon: 'schedule' },
    damaged: { label: 'Damaged', icon: 'broken_image' },
    wrong_order: { label: 'Wrong Order', icon: 'shuffle' },
    rude: { label: 'Rude', icon: 'sentiment_dissatisfied' },
    bike_problem: { label: 'Bike Problem', icon: 'pedal_bike' },
    other: { label: 'Other', icon: 'help_outline' }
  };

  readonly reporterTypeLabels: Record<IssueReporterType, string> = {
    customer: 'Customer',
    deliverer: 'Deliverer',
    restaurant: 'Restaurant'
  };

  private readonly destroy$ = new Subject<void>();

  constructor(
    private issueService: IssueService,
    private cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.loadIssues();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  onResolvedFilterChange(value: ResolvedFilter): void {
    this.resolvedFilter = value;
    this.loadIssues();
  }

  onCategoryFilterChange(value: CategoryFilter): void {
    this.categoryFilter = value;
    this.loadIssues();
  }

  selectIssue(issue: Issue): void {
    this.selectedIssue = issue;
  }

  trackByIssueId(_index: number, issue: Issue): string {
    return issue.id;
  }

  getCategoryInfo(category: IssueCategory): CategoryInfo {
    return this.categoryInfoMap[category];
  }

  getReporterTypeLabel(reporterType: IssueReporterType): string {
    return this.reporterTypeLabels[reporterType];
  }

  formatDate(date: Date): string {
    return new Date(date).toLocaleString();
  }

  private loadIssues(): void {
    this.loading = true;
    this.error = null;

    const filters: IssueFilters = {};

    if (this.resolvedFilter !== 'all') {
      filters.resolved = this.resolvedFilter === 'resolved';
    }

    if (this.categoryFilter !== 'all') {
      filters.category = this.categoryFilter;
    }

    this.issueService.getIssues(filters)
      .pipe(
        takeUntil(this.destroy$),
        finalize(() => {
          this.loading = false;
          this.cdr.markForCheck();
        })
      )
      .subscribe({
        next: (issues) => {
          this.issues = issues;
          // Clear selection if the selected issue is no longer in the filtered list
          if (this.selectedIssue && !issues.find(i => i.id === this.selectedIssue?.id)) {
            this.selectedIssue = null;
          }
        },
        error: (err) => {
          this.error = 'Failed to load issues. Please try again.';
          console.error('Error loading issues:', err);
        }
      });
  }
}
