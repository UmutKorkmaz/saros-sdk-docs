/**
 * ReportGenerator - Generate IL reports and visualizations
 */

import { logger } from './utils/logger';
import * as fs from 'fs';
import * as path from 'path';

export interface Position {
  pool: string;
  type: 'AMM' | 'DLMM';
  value: number;
  il: number;
  fees: number;
  duration: number;
}

export interface ReportConfig {
  positions: Position[];
  format: 'json' | 'html' | 'csv';
  includeCharts: boolean;
  outputDir?: string;
}

export interface ReportResult {
  summary: {
    totalValue: number;
    averageIL: number;
    totalFees: number;
    netPnL: number;
  };
  filepath: string;
  generated: Date;
}

export class ReportGenerator {
  private outputDir: string;

  constructor(outputDir: string = './reports') {
    this.outputDir = outputDir;
    this.ensureOutputDir();
    logger.info('ReportGenerator initialized');
  }

  /**
   * Generate comprehensive IL report
   */
  async generateReport(config: ReportConfig): Promise<ReportResult> {
    const { positions, format, includeCharts } = config;
    
    // Calculate summary statistics
    const summary = this.calculateSummary(positions);
    
    // Generate report content
    let content: string;
    let filename: string;
    
    switch (format) {
      case 'json':
        content = this.generateJSONReport(positions, summary);
        filename = `il-report-${Date.now()}.json`;
        break;
        
      case 'html':
        content = this.generateHTMLReport(positions, summary, includeCharts);
        filename = `il-report-${Date.now()}.html`;
        break;
        
      case 'csv':
        content = this.generateCSVReport(positions);
        filename = `il-report-${Date.now()}.csv`;
        break;
        
      default:
        throw new Error(`Unsupported format: ${format}`);
    }
    
    // Save report
    const filepath = path.join(this.outputDir, filename);
    fs.writeFileSync(filepath, content);
    
    logger.info(`Report generated: ${filepath}`);
    
    return {
      summary,
      filepath,
      generated: new Date()
    };
  }

  /**
   * Calculate summary statistics
   */
  private calculateSummary(positions: Position[]): any {
    const totalValue = positions.reduce((sum, p) => sum + p.value, 0);
    const totalIL = positions.reduce((sum, p) => sum + (p.value * p.il / 100), 0);
    const totalFees = positions.reduce((sum, p) => sum + p.fees, 0);
    const averageIL = positions.reduce((sum, p) => sum + p.il, 0) / positions.length;
    const netPnL = totalFees - totalIL;
    
    return {
      totalValue,
      averageIL,
      totalFees,
      netPnL
    };
  }

  /**
   * Generate JSON report
   */
  private generateJSONReport(positions: Position[], summary: any): string {
    return JSON.stringify({
      generated: new Date().toISOString(),
      summary,
      positions,
      analysis: {
        ilCoverage: (summary.totalFees / (summary.totalValue * summary.averageIL / 100)) * 100,
        profitablePositions: positions.filter(p => p.fees > (p.value * p.il / 100)).length,
        totalPositions: positions.length
      }
    }, null, 2);
  }

  /**
   * Generate HTML report
   */
  private generateHTMLReport(positions: Position[], summary: any, includeCharts: boolean): string {
    const chartSection = includeCharts ? this.generateChartHTML(positions) : '';
    
    return `<!DOCTYPE html>
<html>
<head>
  <title>Impermanent Loss Report</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 20px; }
    h1 { color: #333; }
    table { border-collapse: collapse; width: 100%; margin: 20px 0; }
    th, td { border: 1px solid #ddd; padding: 12px; text-align: left; }
    th { background-color: #f2f2f2; }
    .summary { background-color: #f9f9f9; padding: 15px; border-radius: 5px; margin: 20px 0; }
    .positive { color: green; }
    .negative { color: red; }
  </style>
</head>
<body>
  <h1>Impermanent Loss Report</h1>
  <div class="summary">
    <h2>Summary</h2>
    <p><strong>Total Portfolio Value:</strong> $${summary.totalValue.toFixed(2)}</p>
    <p><strong>Average IL:</strong> ${summary.averageIL.toFixed(2)}%</p>
    <p><strong>Total Fees Earned:</strong> $${summary.totalFees.toFixed(2)}</p>
    <p><strong>Net P&L:</strong> <span class="${summary.netPnL >= 0 ? 'positive' : 'negative'}">$${summary.netPnL.toFixed(2)}</span></p>
  </div>
  
  <h2>Position Details</h2>
  <table>
    <thead>
      <tr>
        <th>Pool</th>
        <th>Type</th>
        <th>Value</th>
        <th>IL %</th>
        <th>IL USD</th>
        <th>Fees</th>
        <th>Net</th>
        <th>Duration</th>
      </tr>
    </thead>
    <tbody>
      ${positions.map(p => {
        const ilUSD = p.value * p.il / 100;
        const net = p.fees - ilUSD;
        return `
        <tr>
          <td>${p.pool}</td>
          <td>${p.type}</td>
          <td>$${p.value.toFixed(2)}</td>
          <td>${p.il.toFixed(2)}%</td>
          <td>$${ilUSD.toFixed(2)}</td>
          <td>$${p.fees.toFixed(2)}</td>
          <td class="${net >= 0 ? 'positive' : 'negative'}">$${net.toFixed(2)}</td>
          <td>${p.duration} days</td>
        </tr>`;
      }).join('')}
    </tbody>
  </table>
  
  ${chartSection}
  
  <footer>
    <p><small>Generated: ${new Date().toLocaleString()}</small></p>
  </footer>
</body>
</html>`;
  }

  /**
   * Generate CSV report
   */
  private generateCSVReport(positions: Position[]): string {
    const headers = 'Pool,Type,Value,IL%,IL_USD,Fees,Net,Duration';
    const rows = positions.map(p => {
      const ilUSD = p.value * p.il / 100;
      const net = p.fees - ilUSD;
      return `${p.pool},${p.type},${p.value},${p.il},${ilUSD},${p.fees},${net},${p.duration}`;
    });
    
    return [headers, ...rows].join('\n');
  }

  /**
   * Generate chart HTML section
   */
  private generateChartHTML(positions: Position[]): string {
    // Simplified chart generation
    // In production, use a proper charting library
    
    return `
    <h2>Visual Analysis</h2>
    <div style="margin: 20px 0;">
      <canvas id="ilChart" width="600" height="300" style="border: 1px solid #ddd;"></canvas>
    </div>
    <script>
      // Chart rendering code would go here
      console.log('Charts would be rendered here with Chart.js or similar');
    </script>`;
  }

  /**
   * Ensure output directory exists
   */
  private ensureOutputDir(): void {
    if (!fs.existsSync(this.outputDir)) {
      fs.mkdirSync(this.outputDir, { recursive: true });
    }
  }
}