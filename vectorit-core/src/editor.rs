use crate::types::{Region, Result, SegEdit, Segmentation, VectorItError};

/// Maximum number of undo entries to keep in memory.
const MAX_UNDO_STACK: usize = 20;

/// Segmentation editor that supports applying edits with undo/reset.
#[derive(Debug, Clone)]
pub struct SegmentationEditor {
    original: Segmentation,
    current: Segmentation,
    undo_stack: Vec<Segmentation>,
}

impl SegmentationEditor {
    /// Create a new editor from an initial segmentation.
    pub fn new(segmentation: Segmentation) -> Self {
        Self {
            original: segmentation.clone(),
            current: segmentation,
            undo_stack: Vec::new(),
        }
    }

    /// Apply an edit operation to the current segmentation.
    pub fn apply_edit(&mut self, edit: SegEdit) -> Result<()> {
        // Push current state onto undo stack
        if self.undo_stack.len() >= MAX_UNDO_STACK {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.current.clone());

        match edit {
            SegEdit::PaintPixels { pixels, target_region } => {
                self.apply_paint_pixels(pixels, target_region)?;
            }
            SegEdit::MergeRegions { source, target } => {
                self.apply_merge_regions(source, target)?;
            }
            SegEdit::SplitRegion { region_id, split_line } => {
                self.apply_split_region(region_id, split_line)?;
            }
        }

        Ok(())
    }

    /// Undo the last edit. Returns false if undo stack is empty.
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.current = prev;
            true
        } else {
            false
        }
    }

    /// Reset to the original segmentation, clearing undo history.
    pub fn reset(&mut self) {
        self.current = self.original.clone();
        self.undo_stack.clear();
    }

    /// Get a reference to the current segmentation state.
    pub fn get_current(&self) -> &Segmentation {
        &self.current
    }

    /// Returns true if the undo stack has entries.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    fn apply_paint_pixels(&mut self, pixels: Vec<(u32, u32)>, target_region: u32) -> Result<()> {
        let width = self.current.width as usize;
        let height = self.current.height as usize;

        // Verify target region exists
        if !self.current.regions.iter().any(|r| r.id == target_region) {
            return Err(VectorItError::ExportFailed(format!(
                "Target region {} does not exist",
                target_region
            )));
        }

        // Track which regions lose/gain pixels
        let mut affected_regions = std::collections::HashSet::new();
        affected_regions.insert(target_region);

        for (x, y) in &pixels {
            let x = *x as usize;
            let y = *y as usize;
            if x >= width || y >= height {
                continue;
            }
            let idx = y * width + x;
            let old_region = self.current.label_map[idx];
            if old_region != target_region {
                affected_regions.insert(old_region);
                self.current.label_map[idx] = target_region;
            }
        }

        // Recalculate pixel counts for affected regions
        self.recalculate_pixel_counts(&affected_regions);

        Ok(())
    }

    fn apply_merge_regions(&mut self, source: u32, target: u32) -> Result<()> {
        // Verify both regions exist
        let source_exists = self.current.regions.iter().any(|r| r.id == source);
        let target_exists = self.current.regions.iter().any(|r| r.id == target);

        if !source_exists || !target_exists {
            return Err(VectorItError::ExportFailed(format!(
                "Region {} or {} does not exist",
                source, target
            )));
        }

        // Replace all source labels with target
        for label in self.current.label_map.iter_mut() {
            if *label == source {
                *label = target;
            }
        }

        // Get source pixel count to add to target
        let source_count = self
            .current
            .regions
            .iter()
            .find(|r| r.id == source)
            .map(|r| r.pixel_count)
            .unwrap_or(0);

        // Update target pixel count
        if let Some(target_region) = self.current.regions.iter_mut().find(|r| r.id == target) {
            target_region.pixel_count += source_count;
        }

        // Remove source region
        self.current.regions.retain(|r| r.id != source);

        Ok(())
    }

    fn apply_split_region(
        &mut self,
        region_id: u32,
        split_line: (crate::types::Point, crate::types::Point),
    ) -> Result<()> {
        let width = self.current.width as usize;
        let height = self.current.height as usize;

        // Verify region exists
        let region = self
            .current
            .regions
            .iter()
            .find(|r| r.id == region_id)
            .ok_or_else(|| {
                VectorItError::ExportFailed(format!("Region {} does not exist", region_id))
            })?;
        let color_index = region.color_index;

        // Split line divides region pixels into two sides based on which side of the line they fall on
        let (p1, p2) = split_line;
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;

        // Assign a new region id for pixels on one side
        let new_region_id = self
            .current
            .regions
            .iter()
            .map(|r| r.id)
            .max()
            .unwrap_or(0)
            + 1;

        let mut new_count = 0u32;
        let mut old_count = 0u32;

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                if self.current.label_map[idx] != region_id {
                    continue;
                }

                // Cross product to determine which side of the line this pixel is on
                let px = x as f64 + 0.5 - p1.x;
                let py = y as f64 + 0.5 - p1.y;
                let cross = dx * py - dy * px;

                if cross < 0.0 {
                    // Reassign to new region
                    self.current.label_map[idx] = new_region_id;
                    new_count += 1;
                } else {
                    old_count += 1;
                }
            }
        }

        // Update old region pixel count
        if let Some(r) = self.current.regions.iter_mut().find(|r| r.id == region_id) {
            r.pixel_count = old_count;
        }

        // Add new region
        if new_count > 0 {
            self.current.regions.push(Region {
                id: new_region_id,
                color_index,
                pixel_count: new_count,
            });
        }

        Ok(())
    }

    fn recalculate_pixel_counts(&mut self, affected_regions: &std::collections::HashSet<u32>) {
        // Reset counts for affected regions
        for region in self.current.regions.iter_mut() {
            if affected_regions.contains(&region.id) {
                region.pixel_count = 0;
            }
        }

        // Count pixels
        for label in &self.current.label_map {
            if affected_regions.contains(label) {
                if let Some(region) = self.current.regions.iter_mut().find(|r| r.id == *label) {
                    region.pixel_count += 1;
                }
            }
        }
    }
}

/// Find articulation points (pinching artifacts) in a segmentation.
/// Returns pixel coordinates where removing the pixel would disconnect a region.
pub fn find_artifacts(segmentation: &Segmentation) -> Vec<(u32, u32)> {
    let width = segmentation.width as usize;
    let height = segmentation.height as usize;
    let mut artifacts = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let region_id = segmentation.label_map[idx];

            if is_articulation_point(&segmentation.label_map, width, height, x, y, region_id) {
                artifacts.push((x as u32, y as u32));
            }
        }
    }

    artifacts
}

/// Check if a pixel is an articulation point in its region using neighbor connectivity.
fn is_articulation_point(
    label_map: &[u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    region_id: u32,
) -> bool {
    // Get 4-connected neighbors that belong to the same region
    let mut neighbors = Vec::new();
    let offsets: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    for (dx, dy) in &offsets {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
            continue;
        }
        let nidx = ny as usize * width + nx as usize;
        if label_map[nidx] == region_id {
            neighbors.push((nx as usize, ny as usize));
        }
    }

    // If 0 or 1 neighbor, not an articulation point
    if neighbors.len() <= 1 {
        return false;
    }

    // Check if removing this pixel disconnects its neighbors.
    // Do a BFS/DFS from the first neighbor, see if all other neighbors are reachable
    // without passing through (x, y).
    let start = neighbors[0];
    let mut visited = std::collections::HashSet::new();
    visited.insert(start);
    let mut stack = vec![start];

    while let Some((cx, cy)) = stack.pop() {
        for (dx, dy) in &offsets {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let (nux, nuy) = (nx as usize, ny as usize);
            // Skip the pixel being tested
            if nux == x && nuy == y {
                continue;
            }
            if visited.contains(&(nux, nuy)) {
                continue;
            }
            let nidx = nuy * width + nux;
            if label_map[nidx] == region_id {
                visited.insert((nux, nuy));
                stack.push((nux, nuy));
            }
        }
    }

    // If any neighbor is NOT reachable from the first, this is an articulation point
    neighbors.iter().skip(1).any(|n| !visited.contains(n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Region, Segmentation};

    fn make_test_segmentation() -> Segmentation {
        // 4x4 image with 2 regions:
        // Region 0: top-left 2x2
        // Region 1: rest
        #[rustfmt::skip]
        let label_map = vec![
            0, 0, 1, 1,
            0, 0, 1, 1,
            1, 1, 1, 1,
            1, 1, 1, 1,
        ];
        Segmentation {
            regions: vec![
                Region { id: 0, color_index: 0, pixel_count: 4 },
                Region { id: 1, color_index: 1, pixel_count: 12 },
            ],
            label_map,
            width: 4,
            height: 4,
        }
    }

    #[test]
    fn test_paint_pixels() {
        let seg = make_test_segmentation();
        let mut editor = SegmentationEditor::new(seg);

        // Paint pixel (0,0) to region 1
        editor
            .apply_edit(SegEdit::PaintPixels {
                pixels: vec![(0, 0)],
                target_region: 1,
            })
            .unwrap();

        assert_eq!(editor.get_current().label_map[0], 1);
        let r0 = editor.get_current().regions.iter().find(|r| r.id == 0).unwrap();
        assert_eq!(r0.pixel_count, 3);
        let r1 = editor.get_current().regions.iter().find(|r| r.id == 1).unwrap();
        assert_eq!(r1.pixel_count, 13);
    }

    #[test]
    fn test_merge_regions() {
        let seg = make_test_segmentation();
        let mut editor = SegmentationEditor::new(seg);

        editor
            .apply_edit(SegEdit::MergeRegions { source: 0, target: 1 })
            .unwrap();

        // Region 0 should be gone
        assert!(!editor.get_current().regions.iter().any(|r| r.id == 0));
        let r1 = editor.get_current().regions.iter().find(|r| r.id == 1).unwrap();
        assert_eq!(r1.pixel_count, 16);
        // All labels should be 1
        assert!(editor.get_current().label_map.iter().all(|&l| l == 1));
    }

    #[test]
    fn test_undo() {
        let seg = make_test_segmentation();
        let mut editor = SegmentationEditor::new(seg);

        editor
            .apply_edit(SegEdit::PaintPixels {
                pixels: vec![(0, 0)],
                target_region: 1,
            })
            .unwrap();

        assert_eq!(editor.get_current().label_map[0], 1);
        assert!(editor.undo());
        assert_eq!(editor.get_current().label_map[0], 0);
    }

    #[test]
    fn test_undo_empty() {
        let seg = make_test_segmentation();
        let mut editor = SegmentationEditor::new(seg);
        assert!(!editor.undo());
    }

    #[test]
    fn test_reset() {
        let seg = make_test_segmentation();
        let mut editor = SegmentationEditor::new(seg);

        editor
            .apply_edit(SegEdit::MergeRegions { source: 0, target: 1 })
            .unwrap();

        editor.reset();
        assert_eq!(editor.get_current().regions.len(), 2);
        assert_eq!(editor.get_current().label_map[0], 0);
    }
}
