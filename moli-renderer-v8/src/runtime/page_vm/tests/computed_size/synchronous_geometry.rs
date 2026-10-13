//! `LayoutPolicy::Synchronous` geometry reads observe mutations made after the
//! last published layout; `LayoutPolicy::OnDemand` keeps sampling it.

use super::*;

const BOXES: &str = r#"<div id=a style="width:100px;height:20px">A</div>"#;

const RECT_OF: &str = "globalThis.rectOf=id=>{const e=document.getElementById(id);if(!e)return null;const r=e.getBoundingClientRect();return [r.width,r.height]};'installed'";

fn layout_passes(page: &PageVm) -> u64 {
    page.vm().layout_pass_observability_for_test().1
}

fn rect_of(page: &mut PageVm, id: &str) -> anyhow::Result<serde_json::Value> {
    let value = page.vm_mut().eval(&format!(
        "JSON.stringify(rectOf({}))",
        serde_json::to_string(id)?
    ))?;
    Ok(serde_json::from_str(&value)?)
}

fn mutate(page: &mut PageVm, script: &str) -> anyhow::Result<()> {
    page.vm_mut().eval(&format!("{script};'mutated'"))?;
    Ok(())
}

fn page_with_policy(policy: moli_page_types::LayoutPolicy) -> anyhow::Result<PageVm> {
    let mut page = page_with_size_fixture(BOXES)?;
    page.vm_mut().set_layout_policy(policy);
    page.vm_mut().eval(RECT_OF)?;
    Ok(page)
}

const APPEND_B: &str = "const b=document.createElement('div');b.id='b';b.style.cssText='width:200px;height:30px';b.textContent='B';document.body.appendChild(b)";

#[tokio::test(flavor = "current_thread")]
async fn synchronous_geometry_reads_without_mutation_reuse_one_layout() {
    run_page_vm_async_test(async move {
        let mut page = page_with_policy(moli_page_types::LayoutPolicy::Synchronous)?;
        assert_eq!(rect_of(&mut page, "a")?, json!([100, 20]));
        let passes = layout_passes(&page);
        for _ in 0..3 {
            assert_eq!(rect_of(&mut page, "a")?, json!([100, 20]));
            assert_eq!(
                page.vm_mut()
                    .eval("String(document.getElementById('a').offsetWidth)")?,
                "100"
            );
        }
        assert_eq!(layout_passes(&page), passes);
        Ok::<_, anyhow::Error>(())
    })
    .await
    .expect("an unmutated page keeps its published layout");
}

#[tokio::test(flavor = "current_thread")]
async fn synchronous_geometry_reads_observe_dom_and_style_mutations() {
    run_page_vm_async_test(async move {
        let mut page = page_with_policy(moli_page_types::LayoutPolicy::Synchronous)?;
        assert_eq!(rect_of(&mut page, "a")?, json!([100, 20]));
        let passes = layout_passes(&page);

        mutate(&mut page, APPEND_B)?;
        assert_eq!(rect_of(&mut page, "b")?, json!([200, 30]));
        assert_eq!(layout_passes(&page), passes + 1);

        mutate(
            &mut page,
            "document.getElementById('a').style.width='300px'",
        )?;
        assert_eq!(rect_of(&mut page, "a")?, json!([300, 20]));
        assert_eq!(
            page.vm_mut()
                .eval("String(document.getElementById('a').offsetWidth)")?,
            "300"
        );
        assert_eq!(layout_passes(&page), passes + 2);

        // Reading again without a mutation reuses the refreshed layout.
        assert_eq!(rect_of(&mut page, "b")?, json!([200, 30]));
        assert_eq!(layout_passes(&page), passes + 2);
        Ok::<_, anyhow::Error>(())
    })
    .await
    .expect("geometry reads observe mutations under synchronous layout");
}

#[tokio::test(flavor = "current_thread")]
async fn synchronous_geometry_reads_observe_a_viewport_change() {
    run_page_vm_async_test(async move {
        let mut page =
            page_with_size_fixture(r#"<div id=a style="width:50%;height:20px">A</div>"#)?;
        page.vm_mut()
            .set_layout_policy(moli_page_types::LayoutPolicy::Synchronous);
        page.vm_mut().eval(RECT_OF)?;
        page.set_viewport_surface(Some(crate::protocol_types::ViewportSurface {
            inner_width: 400,
            inner_height: 300,
            device_pixel_ratio: 1.0,
            ..Default::default()
        }))?;
        assert_eq!(rect_of(&mut page, "a")?, json!([200, 20]));
        page.set_viewport_surface(Some(crate::protocol_types::ViewportSurface {
            inner_width: 600,
            inner_height: 300,
            device_pixel_ratio: 1.0,
            ..Default::default()
        }))?;
        assert_eq!(rect_of(&mut page, "a")?, json!([300, 20]));
        Ok::<_, anyhow::Error>(())
    })
    .await
    .expect("geometry reads observe a viewport change under synchronous layout");
}

#[tokio::test(flavor = "current_thread")]
async fn on_demand_geometry_reads_keep_sampling_the_published_layout() {
    run_page_vm_async_test(async move {
        let mut page = page_with_policy(moli_page_types::LayoutPolicy::OnDemand)?;
        assert_eq!(rect_of(&mut page, "a")?, json!([100, 20]));
        let passes = layout_passes(&page);

        mutate(&mut page, APPEND_B)?;
        mutate(
            &mut page,
            "document.getElementById('a').style.width='300px'",
        )?;
        assert_eq!(rect_of(&mut page, "b")?, json!([0, 0]));
        assert_eq!(rect_of(&mut page, "a")?, json!([100, 20]));
        assert_eq!(layout_passes(&page), passes);

        publish_size_layout(&mut page)?;
        assert_eq!(rect_of(&mut page, "b")?, json!([200, 30]));
        assert_eq!(rect_of(&mut page, "a")?, json!([300, 20]));
        Ok::<_, anyhow::Error>(())
    })
    .await
    .expect("on-demand geometry follows explicit visual publication only");
}
