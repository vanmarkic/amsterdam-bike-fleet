import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { Issue, IssueCategory } from '../models/fleet.models';

export interface IssueFilters {
  resolved?: boolean;
  bikeId?: string;
  category?: IssueCategory;
}

@Injectable({
  providedIn: 'root'
})
export class IssueService {
  private readonly API_URL = '/api/issues';

  constructor(private http: HttpClient) {}

  getIssues(filters?: IssueFilters): Observable<Issue[]> {
    let params = new HttpParams();

    if (filters?.resolved !== undefined) {
      params = params.set('resolved', filters.resolved.toString());
    }
    if (filters?.bikeId) {
      params = params.set('bikeId', filters.bikeId);
    }
    if (filters?.category) {
      params = params.set('category', filters.category);
    }

    return this.http.get<Issue[]>(this.API_URL, { params });
  }

  getIssueById(id: string): Observable<Issue> {
    return this.http.get<Issue>(`${this.API_URL}/${id}`);
  }
}
