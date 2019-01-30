use crate::Selector;
use crate::selector::SelectorSegment;
use syn::visit::Visit;
use syn::{
    self, Attribute, Ident, Item, ItemConst, ItemFn, ItemTrait, ItemType, Stmt, TraitItem,
    Visibility,
};

trait Name {
    /// Get the name of the item, if it has one.
    fn name(&self) -> Option<&Ident>;

    /// Check if the item is named and matches the sought-after ident.
    fn is_named(&self, ident: &impl PartialEq<Ident>) -> bool {
        if let Some(own) = self.name() {
            ident == own
        } else {
            false
        }
    }
}

trait TryToItem {
    /// Convert the implementing type into a freestanding `syn::Item` if possible,
    /// or return `None`.
    fn to_item(self) -> Option<Item>;
}

trait Attrs {
    /// Get all the attributes directly on this item.
    fn attrs(&self) -> Option<&[Attribute]>;

    fn attrs_mut(&mut self) -> Option<&mut Vec<Attribute>>;

    /// Get a copy of the `cfg` attributes directly on this item so they can
    /// be added to other items.
    fn cfg_attrs(&self) -> Vec<Attribute> {
        self.attrs()
            .map(|attrs| {
                attrs
                    .into_iter()
                    .filter_map(|attr| {
                        if attr.path == syn::parse_str("cfg").ok()? {
                            Some(attr.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Modify this instance by adding the specified attributes. It is acceptable
    /// to do nothing in this function if there is no way to apply those attributes
    fn add_attrs(&mut self, attrs: Vec<Attribute>) {
        if let Some(own_attrs) = self.attrs_mut() {
            own_attrs.extend(attrs);
        }
    }
}

#[derive(Debug)]
pub(crate) struct Search<'a> {
    query: &'a Selector,
    depth: usize,
    pub(crate) results: Vec<Item>,
}

impl<'a> Search<'a> {
    /// Create a new search context starting at the top of the given selector
    pub fn new(query: &'a Selector) -> Self {
        Self {
            query,
            depth: 0,
            results: vec![],
        }
    }

    pub fn search_file(&mut self, file: &syn::File) {
        self.visit_file(file)
    }

    /// Get the currently sought term from the provided query path
    fn term(&self) -> &SelectorSegment {
        self.query.part(self.depth)
    }

    fn can_match(&self) -> bool {
        self.depth == self.query.len() - 1
    }

    /// Start a new search for the next term in the path within the specified item.
    fn search_deeper(&self, item: &syn::Item) -> Self {
        let mut new = Self {
            depth: self.depth + 1,
            ..Search::new(self.query)
        };

        if new.depth < new.query.len() {
            for content in contents_of_item(item) {
                new.visit_item(&content);
            }
        }

        new
    }

    /// Apply attributes to the results and return them
    fn with_attrs(mut self, attrs: Vec<Attribute>) -> Vec<Item> {
        if attrs.is_empty() {
            return self.results;
        }

        for item in &mut self.results {
            item.add_attrs(attrs.clone());
        }

        self.results
    }
}

impl<'a> From<&'a Selector> for Search<'a> {
    fn from(query: &'a Selector) -> Self {
        Search::new(query)
    }
}

impl<'a, 'ast> Visit<'ast> for Search<'a> {
    fn visit_item(&mut self, item: &'ast Item) {
        let search_term = self.term();

        if !item.is_named(search_term) {
            return;
        }

        // If we're on the last term of the path, we can go ahead and match
        // right now.
        if self.can_match() {
            self.results.push(item.clone());
            return;
        }

        if let Item::Trait(trait_item) = item {
            self.depth += 1;
            let new_matches = ItemTraitSearch::new(self).search(trait_item);
            self.results.extend(new_matches);
            self.depth -= 1;
            return;
        }

        self.results
            .extend(self.search_deeper(item).with_attrs(item.cfg_attrs()));
    }
}

struct ItemTraitSearch<'a: 'b, 'b> {
    search: &'b Search<'a>,
    trait_results: Vec<TraitItem>,
    free_results: Vec<Item>,
}

impl<'a: 'b, 'b> ItemTraitSearch<'a, 'b> {
    fn new(search: &'b Search<'a>) -> Self {
        Self {
            search,
            trait_results: Vec::new(),
            free_results: Vec::new(),
        }
    }

    /// Find items matching the provided query inside the given trait. This returns a filtered
    /// impl if one or more items matched.
    fn search(mut self, item_trait: &ItemTrait) -> Vec<Item> {
        for item in &item_trait.items {
            self.visit_trait_item(&item);
        }

        if self.trait_results.is_empty() {
            return self.free_results;
        }

        let mut result = item_trait.clone();
        result.items = self.trait_results;

        std::iter::once(Item::from(result))
            .chain(self.free_results)
            .collect()
    }
}

impl<'a, 'b, 'ast> Visit<'ast> for ItemTraitSearch<'a, 'b> {
    fn visit_trait_item(&mut self, item: &TraitItem) {
        // Return early if the name isn't a match.
        if !item.is_named(self.search.term()) {
            return;
        }

        if self.search.can_match() {
            // We've reached the end of the query path, so we should
            // register this trait item as a hit.
            self.trait_results.push(item.clone());
        } else if let Some(child) = item.clone().to_item() {
            // We haven't reached the end, but we can convert the trait
            // member into a free-standing item to continue the search.
            let child_results = self.search.search_deeper(&child);
            self.free_results.extend(child_results.results);
        }
    }
}

fn contents_of_item(item: &Item) -> Vec<Item> {
    match item {
        Item::ExternCrate(_) => Vec::new(),
        Item::Use(_) => Vec::new(),
        Item::Static(_) => Vec::new(),
        Item::Const(_) => Vec::new(),
        Item::Fn(item_fn) => item_fn
            .block
            .stmts
            .iter()
            .cloned()
            .filter_map(Stmt::to_item)
            .collect(),
        Item::Mod(item_mod) => match &item_mod.content {
            Some((_, nested)) => nested.clone(),
            None => Vec::new(),
        },
        Item::ForeignMod(_) => Vec::new(),
        Item::Type(_) => Vec::new(),
        Item::Existential(_) => Vec::new(),
        Item::Struct(_) => Vec::new(),
        Item::Enum(_) => Vec::new(),
        Item::Union(_) => Vec::new(),
        Item::Trait(item_trait) => item_trait
            .items
            .iter()
            .cloned()
            .filter_map(TraitItem::to_item)
            .collect(),
        Item::TraitAlias(_) => Vec::new(),
        Item::Impl(_) => Vec::new(),
        Item::Macro(_) => Vec::new(),
        Item::Macro2(_) => Vec::new(),
        Item::Verbatim(_) => Vec::new(),
    }
}

impl Name for Item {
    fn name(&self) -> Option<&Ident> {
        match self {
            Item::ExternCrate(item) => match &item.rename {
                Some((_, rename)) => Some(rename),
                None => Some(&item.ident),
            },
            Item::Use(_) => None,
            Item::Static(item) => Some(&item.ident),
            Item::Const(item) => Some(&item.ident),
            Item::Fn(item) => Some(&item.ident),
            Item::Mod(item) => Some(&item.ident),
            Item::ForeignMod(_) => None,
            Item::Type(item) => Some(&item.ident),
            Item::Existential(item) => Some(&item.ident),
            Item::Struct(item) => Some(&item.ident),
            Item::Enum(item) => Some(&item.ident),
            Item::Union(item) => Some(&item.ident),
            Item::Trait(item) => Some(&item.ident),
            Item::TraitAlias(item) => Some(&item.ident),
            Item::Impl(_) => None,
            Item::Macro(item) => item.ident.as_ref(),
            Item::Macro2(item) => Some(&item.ident),
            Item::Verbatim(_) => None,
        }
    }
}

impl Attrs for Item {
    fn attrs(&self) -> Option<&[Attribute]> {
        match self {
            Item::ExternCrate(item) => Some(&item.attrs),
            Item::Use(_) => None,
            Item::Static(item) => Some(&item.attrs),
            Item::Const(item) => Some(&item.attrs),
            Item::Fn(item) => Some(&item.attrs),
            Item::Mod(item) => Some(&item.attrs),
            Item::ForeignMod(_) => None,
            Item::Type(item) => Some(&item.attrs),
            Item::Existential(item) => Some(&item.attrs),
            Item::Struct(item) => Some(&item.attrs),
            Item::Enum(item) => Some(&item.attrs),
            Item::Union(item) => Some(&item.attrs),
            Item::Trait(item) => Some(&item.attrs),
            Item::TraitAlias(item) => Some(&item.attrs),
            Item::Impl(_) => None,
            Item::Macro(item) => Some(&item.attrs),
            Item::Macro2(item) => Some(&item.attrs),
            Item::Verbatim(_) => None,
        }
    }

    fn attrs_mut(&mut self) -> Option<&mut Vec<Attribute>> {
        match self {
            Item::ExternCrate(item) => Some(&mut item.attrs),
            Item::Use(_) => None,
            Item::Static(item) => Some(&mut item.attrs),
            Item::Const(item) => Some(&mut item.attrs),
            Item::Fn(item) => Some(&mut item.attrs),
            Item::Mod(item) => Some(&mut item.attrs),
            Item::ForeignMod(_) => None,
            Item::Type(item) => Some(&mut item.attrs),
            Item::Existential(item) => Some(&mut item.attrs),
            Item::Struct(item) => Some(&mut item.attrs),
            Item::Enum(item) => Some(&mut item.attrs),
            Item::Union(item) => Some(&mut item.attrs),
            Item::Trait(item) => Some(&mut item.attrs),
            Item::TraitAlias(item) => Some(&mut item.attrs),
            Item::Impl(_) => None,
            Item::Macro(item) => Some(&mut item.attrs),
            Item::Macro2(item) => Some(&mut item.attrs),
            Item::Verbatim(_) => None,
        }
    }
}

impl Name for TraitItem {
    fn name(&self) -> Option<&Ident> {
        match self {
            TraitItem::Method(item) => Some(&item.sig.ident),
            TraitItem::Const(item) => Some(&item.ident),
            TraitItem::Type(item) => Some(&item.ident),
            TraitItem::Macro(_) => None,
            TraitItem::Verbatim(_) => None,
        }
    }
}

impl TryToItem for TraitItem {
    fn to_item(self) -> Option<Item> {
        match self {
            TraitItem::Const(item) => Some(Item::Const(ItemConst {
                attrs: item.attrs,
                vis: Visibility::Inherited,
                const_token: item.const_token,
                ident: item.ident,
                colon_token: item.colon_token,
                ty: Box::new(item.ty),
                eq_token: item.default.as_ref()?.0,
                expr: Box::new(item.default?.1),
                semi_token: item.semi_token,
            })),
            TraitItem::Method(item) => Some(Item::Fn(ItemFn {
                attrs: item.attrs,
                vis: Visibility::Inherited,
                constness: item.sig.constness,
                unsafety: item.sig.unsafety,
                asyncness: item.sig.asyncness,
                abi: item.sig.abi,
                ident: item.sig.ident,
                decl: Box::new(item.sig.decl),
                block: Box::new(item.default?),
            })),
            TraitItem::Type(item) => Some(Item::Type(ItemType {
                attrs: item.attrs,
                vis: Visibility::Inherited,
                type_token: item.type_token,
                ident: item.ident,
                generics: item.generics,
                eq_token: item.default.as_ref()?.0,
                ty: Box::new(item.default?.1),
                semi_token: item.semi_token,
            })),
            TraitItem::Macro(_) => None,
            TraitItem::Verbatim(_) => None,
        }
    }
}

impl TryToItem for Stmt {
    fn to_item(self) -> Option<Item> {
        if let Stmt::Item(item) = self {
            Some(item)
        } else {
            None
        }
    }
}
