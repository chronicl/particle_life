use bevy::math::{DVec2, Vec2};
use bevy_vello::{
    vello::{
        kurbo::{self, CubicBez, Dashes, Point, Shape},
        peniko::{self, BrushRef},
    },
    VelloScene,
};

pub trait SceneExt {
    fn builder<'a>(&'a mut self) -> DrawBuilder<'a>;
}

impl SceneExt for VelloScene {
    fn builder(&mut self) -> DrawBuilder {
        DrawBuilder {
            scene: self,
            stroke: None,
            fill: None,
            transform: kurbo::Affine::default(),
            stroke_brush: peniko::Color::WHITE.into(),
            fill_brush: peniko::Color::WHITE.into(),
            stroke_brush_transform: None,
            fill_brush_transform: None,
        }
    }
}

pub struct DrawBuilder<'a> {
    scene: &'a mut VelloScene,
    stroke: Option<&'a kurbo::Stroke>,
    fill: Option<peniko::Fill>,
    transform: kurbo::Affine,
    stroke_brush: BrushRef<'a>,
    fill_brush: BrushRef<'a>,
    stroke_brush_transform: Option<kurbo::Affine>,
    fill_brush_transform: Option<kurbo::Affine>,
}

impl<'a> DrawBuilder<'a> {
    pub fn with_stroke(mut self, stroke: &'a kurbo::Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub fn with_default_stroke(mut self) -> Self {
        self.stroke = Some(DEFAULT_STROKE);
        self
    }

    pub fn with_fill(mut self, fill: peniko::Fill) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn with_default_fill(mut self) -> Self {
        self.fill = Some(peniko::Fill::NonZero);
        self
    }

    pub fn with_transform(mut self, transform: kurbo::Affine) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_stroke_brush(mut self, brush: impl Into<BrushRef<'a>>) -> Self {
        self.stroke_brush = brush.into();
        self
    }

    pub fn with_stroke_color(mut self, color: peniko::Color) -> Self {
        self.stroke_brush = color.into();
        self
    }

    pub fn with_fill_brush(mut self, brush: impl Into<BrushRef<'a>>) -> Self {
        self.fill_brush = brush.into();
        self
    }

    pub fn with_fill_color(mut self, color: peniko::Color) -> Self {
        self.fill_brush = color.into();
        self
    }

    pub fn with_stroke_brush_transform(mut self, brush_transform: kurbo::Affine) -> Self {
        self.stroke_brush_transform = Some(brush_transform);
        self
    }

    pub fn with_fill_brush_transform(mut self, brush_transform: kurbo::Affine) -> Self {
        self.fill_brush_transform = Some(brush_transform);
        self
    }

    pub fn draw(self, shape: impl Shape) {
        if let Some(fill) = self.fill {
            self.scene.fill(
                fill,
                self.transform,
                self.fill_brush,
                self.fill_brush_transform,
                &shape,
            );
        }
        if let Some(stroke) = self.stroke {
            self.scene.stroke(
                stroke,
                self.transform,
                self.stroke_brush.clone(),
                self.stroke_brush_transform,
                &shape,
            );
        }
    }

    pub fn draw_circle(self, center: DVec2, radius: f64) {
        self.draw(kurbo::Circle::new(center.to_point(), radius));
    }

    pub fn draw_line(self, start: DVec2, end: DVec2) {
        self.draw(kurbo::Line::new(start.to_point(), end.to_point()));
    }

    pub fn draw_bezier_curve(self, points: Vec<DVec2>) {
        let path = build_bezier_curve(points);
        self.draw(path);
    }
}

const DEFAULT_STROKE: &'static kurbo::Stroke = &kurbo::Stroke {
    width: 1.0,
    join: kurbo::Join::Round,
    miter_limit: 4.0,
    start_cap: kurbo::Cap::Round,
    end_cap: kurbo::Cap::Round,
    dash_pattern: Dashes::new_const(),
    dash_offset: 0.0,
};

pub fn build_bezier_curve(mut points: Vec<DVec2>) -> kurbo::BezPath {
    let mut cubic_bezier_segments = Vec::new();
    // add the first 3 points to the end to close the loop
    points.push(points[0]);
    points.push(points[1]);
    points.push(points[2]);
    for p in points.windows(4) {
        let p1_tangent = (p[2] - p[0]) / 6.;
        let p2_tangent = (p[3] - p[1]) / 6.;
        let (c1, c2) = (p[1] + p1_tangent, p[2] - p2_tangent);
        cubic_bezier_segments.push(CubicBez::new(
            p[1].to_point(),
            c1.to_point(),
            c2.to_point(),
            p[2].to_point(),
        ));
    }

    kurbo::BezPath::from_path_segments(cubic_bezier_segments.into_iter().map(Into::into))
}

pub trait ToPoint {
    fn to_point(self) -> Point;
}

impl ToPoint for Vec2 {
    fn to_point(self) -> Point {
        Point::new(self.x as f64, self.y as f64)
    }
}

impl ToPoint for DVec2 {
    fn to_point(self) -> Point {
        Point::new(self.x, self.y)
    }
}
