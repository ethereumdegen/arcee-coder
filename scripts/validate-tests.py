#!/usr/bin/env python3
"""
Moderac Test Validator
Validates moderac test files against the template and schema
"""

import json
import re
import sys
from pathlib import Path
from typing import Dict, List, Tuple, Any

class ModeracTestValidator:
    def __init__(self):
        self.errors = []
        self.warnings = []
        self.test_count = 0
        self.valid_count = 0
    
    def validate(self, test_path: Path) -> bool:
        """Validate a single test file"""
        self.test_count += 1
        
        if not test_path.exists():
            self.errors.append(f"Test file not found: {test_path}")
            return False
        
        try:
            with open(test_path, 'r') as f:
                content = f.read()
            
            # Check for required frontmatter
            if '---' not in content[:100]:
                self.errors.append(f"Missing YAML frontmatter in {test_path}")
                return False
            
            # Extract frontmatter
            frontmatter = self.extract_frontmatter(content)
            if not frontmatter:
                self.errors.append(f"Could not parse frontmatter in {test_path}")
                return False
            
            # Validate frontmatter fields
            self.validate_frontmatter(frontmatter, test_path)
            
            # Check for required sections
            if '## Description' not in content:
                self.warnings.append(f"Missing '## Description' section in {test_path}")
            
            if '### Scenario' not in content:
                self.warnings.append(f"Missing '### Scenario' section in {test_path}")
            
            if self.errors:
                return False
            
            self.valid_count += 1
            return True
            
        except Exception as e:
            self.errors.append(f"Error reading {test_path}: {str(e)}")
            return False
    
    def extract_frontmatter(self, content: str) -> Dict[str, Any]:
        """Extract YAML frontmatter from markdown content"""
        try:
            # Find the first '---' delimiter
            start = content.find('---')
            if start == -1:
                return None
            
            # Find the end of frontmatter
            end = content.find('---', start + 3)
            if end == -1:
                return None
            
            # Extract and parse frontmatter
            frontmatter_str = content[start + 3:end].strip()
            # Simple YAML-like parsing (for demo)
            frontmatter = {}
            for line in frontmatter_str.split('\n'):
                if ':' in line:
                    key, value = line.split(':', 1)
                    frontmatter[key.strip()] = value.strip().strip('"').strip("'")
            
            return frontmatter
        except Exception as e:
            print(f"Error extracting frontmatter: {e}")
            return None
    
    def validate_frontmatter(self, frontmatter: Dict[str, Any], test_path: Path):
        """Validate required frontmatter fields"""
        required_fields = ['name', 'tags', 'skills', 'expected']
        
        for field in required_fields:
            if field not in frontmatter:
                self.errors.append(f"Missing required field '{field}' in {test_path}")
        
        # Validate name format
        if 'name' in frontmatter and not re.match(r'^[a-z0-9-]+$', frontmatter['name']):
            self.errors.append(f"Invalid name format in {test_path}: {frontmatter['name']}")
        
        # Validate tags
        if 'tags' in frontmatter:
            tags = [t.strip() for t in frontmatter['tags'].split(',') if t.strip()]
            if not tags:
                self.errors.append(f"No valid tags found in {test_path}")
        
        # Validate skills
        if 'skills' in frontmatter:
            skills = [s.strip() for s in frontmatter['skills'].split(',') if s.strip()]
            if not skills:
                self.errors.append(f"No valid skills found in {test_path}")
    
    def print_report(self):
        """Print validation report"""
        print("\n" + "="*60)
        print("MODERAC TEST VALIDATION REPORT")
        print("="*60)
        print(f"Total tests checked: {self.test_count}")
        print(f"Valid tests: {self.valid_count}")
        print(f"Errors: {len(self.errors)}")
        print(f"Warnings: {len(self.warnings)}")
        print()
        
        if self.errors:
            print("ERRORS:")
            for error in self.errors[:10]:  # Show first 10 errors
                print(f"  - {error}")
            if len(self.errors) > 10:
                print(f"  ... and {len(self.errors) - 10} more errors")
        
        if self.warnings:
            print("\nWARNINGS:")
            for warning in self.warnings[:10]:
                print(f"  - {warning}")
            if len(self.warnings) > 10:
                print(f"  ... and {len(self.warnings) - 10} more warnings")
        
        print("\n" + "="*60)
        
        if self.errors:
            print("VALIDATION FAILED")
            sys.exit(1)
        else:
            print("VALIDATION PASSED")
            sys.exit(0)

def main():
    validator = ModeracTestValidator()
    
    # Validate all tests in moderac-tests directory
    test_dir = Path("/home/andy/ai/arcee-coder/moderac-tests")
    
    if not test_dir.exists():
        print(f"Test directory not found: {test_dir}")
        sys.exit(1)
    
    # Validate each markdown file
    for test_file in test_dir.glob("*.md"):
        validator.validate(test_file)
    
    validator.print_report()

if __name__ == "__main__":
    main()